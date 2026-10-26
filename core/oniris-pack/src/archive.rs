//! Deterministic `.onirispack` archives and safe extraction.
//!
//! A `.onirispack` is a zip whose root contains `manifest.json`, the audio and
//! art referenced by the manifest, and optional `LICENSE`/`README.md` files.
//! Archives are untrusted input: extraction rejects path traversal, symlinks,
//! duplicate names, unsupported file types, oversized entries, and manifests
//! that reference missing assets.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, System, ZipArchive, ZipWriter};

use crate::{is_safe_relative_path, Manifest, PackError};

/// Maximum number of entries (files plus directories) in an archive.
const MAX_ENTRIES: usize = 128;
/// Maximum size of a single uncompressed entry.
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum total uncompressed size of an archive.
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
/// The manifest is small; reject anything that looks like a bomb.
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
/// Maximum size of the compressed archive itself.
const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_PATH_LEN: usize = 1024;
const MAX_SEGMENT_LEN: usize = 255;

#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    pub manifest: Manifest,
    /// Size of the archive file in bytes.
    pub bytes: u64,
    /// Lowercase hex SHA-256 of the archive file.
    pub sha256: String,
    /// Number of file entries, excluding directory entries.
    pub files: usize,
}

/// Package a validated pack directory as a deterministic `.onirispack`.
pub fn pack_dir(dir: &Path, output: &Path) -> Result<ArchiveInfo, PackError> {
    let manifest = Manifest::load(dir)?;
    manifest.validate_files(dir)?;

    let mut entries = BTreeSet::new();
    entries.insert("manifest.json".to_string());
    for asset in manifest.asset_paths() {
        entries.insert(asset.to_string());
    }
    for optional in ["LICENSE", "README.md"] {
        if dir.join(optional).is_file() {
            entries.insert(optional.to_string());
        }
    }

    let mut total = 0u64;
    for name in &entries {
        validate_entry_name(name, false)?;
        let path = dir.join(name);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(PackError::Validation(format!(
                "refusing to package symlink {name:?}"
            )));
        }
        if !metadata.is_file() {
            return Err(PackError::Validation(format!(
                "pack entry {name:?} is not a regular file"
            )));
        }
        total = total.saturating_add(metadata.len());
        if metadata.len() > MAX_ENTRY_BYTES {
            return Err(PackError::Validation(format!(
                "pack entry {name:?} exceeds the {MAX_ENTRY_BYTES} byte limit"
            )));
        }
        if total > MAX_TOTAL_BYTES {
            return Err(PackError::Validation(format!(
                "pack exceeds the {MAX_TOTAL_BYTES} byte archive limit"
            )));
        }
    }

    let parent = parent_dir(output);
    fs::create_dir_all(parent)?;
    let mut temp = NamedTempFile::new_in(parent)?;

    {
        let mut zip = ZipWriter::new(temp.as_file_mut());
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6))
            .last_modified_time(DateTime::DEFAULT)
            .system(System::Unix)
            .unix_permissions(0o644);

        for name in &entries {
            zip.start_file(name.as_str(), options).map_err(zip_error)?;
            let mut file = File::open(dir.join(name))?;
            io::copy(&mut file, &mut zip)?;
        }
        zip.finish().map_err(zip_error)?;
    }

    temp.as_file().sync_all()?;
    temp.persist(output)
        .map_err(|error| PackError::Io(error.error))?;
    set_archive_permissions(output)?;

    let bytes = fs::metadata(output)?.len();
    let sha256 = sha256_file(output)?;
    let files = entries
        .iter()
        .filter(|name| name.as_str() != "manifest.json")
        .count();

    Ok(ArchiveInfo {
        manifest,
        bytes,
        sha256,
        files,
    })
}

/// Validate an archive, including every entry's CRC, without extracting it.
pub fn verify_archive(archive: &Path) -> Result<ArchiveInfo, PackError> {
    let (manifest, files) = process_archive(archive, None)?;
    archive_info(archive, manifest, files)
}

/// Validate an archive and extract it into a new (or empty) directory.
pub fn unpack_archive(archive: &Path, dest: &Path) -> Result<ArchiveInfo, PackError> {
    let dest_exists = dest.exists();
    if dest_exists {
        if !dest.is_dir() {
            return Err(PackError::Validation(format!(
                "destination {} exists and is not a directory",
                dest.display()
            )));
        }
        if fs::read_dir(dest)?.next().is_some() {
            return Err(PackError::Validation(format!(
                "destination {} is not empty",
                dest.display()
            )));
        }
    }

    let parent = parent_dir(dest);
    fs::create_dir_all(parent)?;
    let temp = TempDir::new_in(parent)?;
    let (manifest, files) = process_archive(archive, Some(temp.path()))?;

    // Only remove an existing empty destination once extraction has succeeded,
    // so a bad archive leaves the caller's directory untouched.
    if dest_exists {
        fs::remove_dir(dest)?;
    }
    let extracted = temp.keep();
    if let Err(error) = fs::rename(&extracted, dest) {
        let _ = fs::remove_dir_all(&extracted);
        return Err(PackError::Io(error));
    }

    archive_info(archive, manifest, files)
}

/// Lowercase hex SHA-256 of a file's bytes.
pub fn sha256_file(path: &Path) -> Result<String, PackError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

fn process_archive(archive: &Path, dest: Option<&Path>) -> Result<(Manifest, usize), PackError> {
    let size = fs::metadata(archive)?.len();
    if size > MAX_ARCHIVE_BYTES {
        return Err(PackError::Archive(format!(
            "archive is {size} bytes; the limit is {MAX_ARCHIVE_BYTES}"
        )));
    }

    let file = File::open(archive)?;
    let mut zip = ZipArchive::new(file).map_err(zip_error)?;
    if zip.len() > MAX_ENTRIES {
        return Err(PackError::Archive(format!(
            "archive has {} entries; the limit is {MAX_ENTRIES}",
            zip.len()
        )));
    }

    let mut names = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut manifest_bytes = None;
    let mut total = 0u64;
    let mut actual_total = 0u64;

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(zip_error)?;
        let raw_name = entry.name().map_err(zip_error)?.into_owned();
        let is_dir = entry.is_dir();
        let name = raw_name.strip_suffix('/').unwrap_or(&raw_name);
        validate_entry_name(name, is_dir)?;

        if !names.insert(name.to_string()) {
            return Err(PackError::Archive(format!(
                "duplicate archive entry {name:?}"
            )));
        }
        if is_dir {
            continue;
        }
        if name != "manifest.json" {
            files.insert(name.to_string());
        }

        if entry.encrypted() {
            return Err(PackError::Archive(format!(
                "archive entry {name:?} is encrypted"
            )));
        }
        if entry.is_symlink() {
            return Err(PackError::Archive(format!(
                "archive entry {name:?} is a symlink"
            )));
        }

        let declared = entry.size();
        if declared > MAX_ENTRY_BYTES {
            return Err(PackError::Archive(format!(
                "archive entry {name:?} is {declared} bytes; the limit is {MAX_ENTRY_BYTES}"
            )));
        }
        total = total.saturating_add(declared);
        if total > MAX_TOTAL_BYTES {
            return Err(PackError::Archive(format!(
                "archive expands past the {MAX_TOTAL_BYTES} byte limit"
            )));
        }

        let limit = if name == "manifest.json" {
            MAX_MANIFEST_BYTES
        } else {
            MAX_ENTRY_BYTES
        };
        let mut reader = entry.by_ref().take(limit + 1);

        let copied = if name == "manifest.json" {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
            if bytes.len() as u64 > limit {
                return Err(PackError::Archive(format!(
                    "manifest exceeds the {MAX_MANIFEST_BYTES} byte limit"
                )));
            }
            let copied = bytes.len() as u64;
            if let Some(dest) = dest {
                fs::write(dest.join(name), &bytes)?;
            }
            manifest_bytes = Some(bytes);
            copied
        } else if let Some(dest) = dest {
            let path = dest.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&path)?;
            io::copy(&mut reader, &mut output)?
        } else {
            io::copy(&mut reader, &mut io::sink())?
        };

        if copied > limit {
            return Err(PackError::Archive(format!(
                "archive entry {name:?} exceeds the {limit} byte limit"
            )));
        }
        actual_total = actual_total.saturating_add(copied);
        if actual_total > MAX_TOTAL_BYTES {
            return Err(PackError::Archive(format!(
                "archive expands past the {MAX_TOTAL_BYTES} byte limit"
            )));
        }
    }

    let manifest_bytes = manifest_bytes
        .ok_or_else(|| PackError::Archive("archive is missing a root manifest.json".to_string()))?;
    let text = std::str::from_utf8(&manifest_bytes)
        .map_err(|error| PackError::Archive(format!("manifest is not UTF-8: {error}")))?;
    let manifest = Manifest::from_json_str(text)?;

    for asset in manifest.asset_paths() {
        if !files.contains(asset) {
            return Err(PackError::Archive(format!(
                "manifest references missing archive asset {asset:?}"
            )));
        }
    }

    Ok((manifest, files.len()))
}

fn archive_info(
    archive: &Path,
    manifest: Manifest,
    files: usize,
) -> Result<ArchiveInfo, PackError> {
    let bytes = fs::metadata(archive)?.len();
    let sha256 = sha256_file(archive)?;
    Ok(ArchiveInfo {
        manifest,
        bytes,
        sha256,
        files,
    })
}

fn validate_entry_name(name: &str, is_dir: bool) -> Result<(), PackError> {
    if name.len() > MAX_PATH_LEN {
        return Err(PackError::Archive(format!(
            "archive entry path is longer than {MAX_PATH_LEN} bytes"
        )));
    }
    if !is_safe_relative_path(name) {
        return Err(PackError::Archive(format!(
            "unsafe archive entry path {name:?}"
        )));
    }
    if name
        .split('/')
        .any(|segment| segment.len() > MAX_SEGMENT_LEN)
    {
        return Err(PackError::Archive(format!(
            "archive entry {name:?} has a segment longer than {MAX_SEGMENT_LEN} bytes"
        )));
    }
    if name
        .chars()
        .any(|c| matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return Err(PackError::Archive(format!(
            "archive entry {name:?} contains a reserved character"
        )));
    }
    if !is_dir && !is_allowed_file(name) {
        return Err(PackError::Archive(format!(
            "archive entry {name:?} has an unsupported file type"
        )));
    }
    Ok(())
}

fn is_allowed_file(name: &str) -> bool {
    if matches!(name, "manifest.json" | "LICENSE" | "README.md") {
        return true;
    }
    // Only accept the root-level manifest; a nested manifest is just another
    // unsupported JSON file.
    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    matches!(
        extension.as_deref(),
        Some("opus" | "ogg" | "wav" | "webp" | "png" | "jpg" | "jpeg")
    )
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn zip_error(error: zip::result::ZipError) -> PackError {
    PackError::Archive(error.to_string())
}

#[cfg(unix)]
fn set_archive_permissions(path: &Path) -> Result<(), PackError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_archive_permissions(_path: &Path) -> Result<(), PackError> {
    Ok(())
}

/// Extension used for distributed packs.
pub const ARCHIVE_EXTENSION: &str = "onirispack";

/// Suggested output path for a pack id.
pub fn archive_path_for(id: &str) -> PathBuf {
    PathBuf::from(format!("{id}.{ARCHIVE_EXTENSION}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn manifest_json(id: &str) -> String {
        serde_json::json!({
            "schemaVersion": 1,
            "id": id,
            "name": { "en": "Archive Test" },
            "description": { "en": "Archive round-trip fixture." },
            "version": "1.0.0",
            "license": "CC0-1.0",
            "art": "art/cover.webp",
            "beds": [{
                "id": "rain",
                "file": "audio/rain.opus",
                "loop": { "startSample": 0, "endSample": 48000 }
            }],
            "events": [{
                "id": "thunder",
                "files": ["audio/thunder.opus"],
                "maxVoices": 1
            }]
        })
        .to_string()
    }

    fn make_pack(dir: &Path) -> String {
        std::fs::create_dir_all(dir.join("audio")).unwrap();
        std::fs::create_dir_all(dir.join("art")).unwrap();
        let json = manifest_json("test-rain");
        std::fs::write(dir.join("manifest.json"), &json).unwrap();
        std::fs::write(dir.join("audio/rain.opus"), b"rain audio").unwrap();
        std::fs::write(dir.join("audio/thunder.opus"), b"thunder audio").unwrap();
        std::fs::write(dir.join("art/cover.webp"), b"cover").unwrap();
        std::fs::write(dir.join("LICENSE"), b"CC0-1.0").unwrap();
        std::fs::write(dir.join("README.md"), b"# Archive Test").unwrap();
        json
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::DEFAULT)
            .system(System::Unix);
        for (name, data) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn packs_verifies_and_unpacks_round_trip() {
        let source = tempfile::tempdir().unwrap();
        let json = make_pack(source.path());
        let output = tempfile::tempdir().unwrap();
        let archive = output.path().join("test-rain.onirispack");

        let packed = pack_dir(source.path(), &archive).unwrap();
        assert_eq!(packed.manifest.id, "test-rain");
        assert_eq!(packed.files, 5);
        assert!(packed.bytes > 0);
        assert_eq!(packed.sha256.len(), 64);

        let verified = verify_archive(&archive).unwrap();
        assert_eq!(verified.sha256, packed.sha256);
        assert_eq!(verified.files, 5);

        let dest = output.path().join("unpacked");
        let unpacked = unpack_archive(&archive, &dest).unwrap();
        assert_eq!(unpacked.manifest.id, "test-rain");
        assert_eq!(unpacked.sha256, packed.sha256);
        assert_eq!(
            std::fs::read(dest.join("audio/rain.opus")).unwrap(),
            b"rain audio"
        );
        assert_eq!(
            std::fs::read(dest.join("manifest.json")).unwrap(),
            json.as_bytes()
        );
        assert_eq!(Manifest::load(&dest).unwrap().id, "test-rain");
    }

    #[test]
    fn packing_is_deterministic() {
        let source = tempfile::tempdir().unwrap();
        make_pack(source.path());
        let output = tempfile::tempdir().unwrap();

        let first = pack_dir(source.path(), &output.path().join("a.onirispack")).unwrap();
        let second = pack_dir(source.path(), &output.path().join("b.onirispack")).unwrap();

        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.bytes, second.bytes);
    }

    #[test]
    fn rejects_path_traversal_entries() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("evil.onirispack");
        write_zip(
            &archive,
            &[
                ("manifest.json", manifest_json("test-rain").as_bytes()),
                ("../evil.opus", b"payload"),
            ],
        );

        let error = verify_archive(&archive).unwrap_err();
        assert!(error.to_string().contains("unsafe archive entry"));
    }

    #[test]
    fn rejects_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("evil.onirispack");
        write_zip(
            &archive,
            &[
                ("manifest.json", manifest_json("test-rain").as_bytes()),
                ("/etc/passwd", b"payload"),
            ],
        );

        assert!(matches!(
            verify_archive(&archive),
            Err(PackError::Archive(_))
        ));
    }

    #[test]
    fn rejects_unsupported_file_types() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("evil.onirispack");
        write_zip(
            &archive,
            &[
                ("manifest.json", manifest_json("test-rain").as_bytes()),
                ("run.sh", b"#!/bin/sh"),
            ],
        );

        let error = verify_archive(&archive).unwrap_err();
        assert!(error.to_string().contains("unsupported file type"));
    }

    #[test]
    fn rejects_missing_assets() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("broken.onirispack");
        write_zip(
            &archive,
            &[("manifest.json", manifest_json("test-rain").as_bytes())],
        );

        let error = verify_archive(&archive).unwrap_err();
        assert!(error.to_string().contains("missing archive asset"));
    }

    #[test]
    fn rejects_malformed_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("broken.onirispack");
        write_zip(&archive, &[("manifest.json", b"not json")]);

        assert!(verify_archive(&archive).is_err());
    }

    #[test]
    fn rejects_non_empty_destination() {
        let source = tempfile::tempdir().unwrap();
        make_pack(source.path());
        let output = tempfile::tempdir().unwrap();
        let archive = output.path().join("test.onirispack");
        pack_dir(source.path(), &archive).unwrap();

        let dest = output.path().join("existing");
        std::fs::create_dir(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"keep").unwrap();

        let error = unpack_archive(&archive, &dest).unwrap_err();
        assert!(error.to_string().contains("not empty"));
        assert!(dest.join("keep.txt").is_file());
    }

    #[test]
    fn bad_archive_leaves_empty_destination_intact() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("broken.onirispack");
        write_zip(&archive, &[("manifest.json", b"not json")]);

        let dest = dir.path().join("existing");
        std::fs::create_dir(&dest).unwrap();

        assert!(unpack_archive(&archive, &dest).is_err());
        assert!(dest.is_dir());
        assert_eq!(std::fs::read_dir(&dest).unwrap().count(), 0);
    }

    #[test]
    fn unpack_accepts_an_empty_destination() {
        let source = tempfile::tempdir().unwrap();
        make_pack(source.path());
        let output = tempfile::tempdir().unwrap();
        let archive = output.path().join("test.onirispack");
        pack_dir(source.path(), &archive).unwrap();

        let dest = output.path().join("existing");
        std::fs::create_dir(&dest).unwrap();
        unpack_archive(&archive, &dest).unwrap();
        assert!(dest.join("audio/rain.opus").is_file());
    }
}
