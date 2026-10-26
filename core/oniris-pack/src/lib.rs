//! Manifest types and validation for Oniris sound packs.
//!
//! A pack is a directory (or later a zip) containing a `manifest.json`, audio
//! files, and optional art. The manifest is deliberately data-only: packs must
//! never contain executable code.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod archive;

pub use archive::{
    archive_path_for, pack_dir, sha256_file, unpack_archive, verify_archive, ArchiveInfo,
    ARCHIVE_EXTENSION,
};

pub const SCHEMA_VERSION: u32 = 1;

/// Validated pack identifier: lowercase ASCII letters, digits, and `-`.
pub const MAX_ID_LEN: usize = 64;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid pack: {0}")]
    Validation(String),
    #[error("archive error: {0}")]
    Archive(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub id: String,
    /// Localized display names keyed by BCP-47 language tag. `en` is required.
    pub name: BTreeMap<String, String>,
    #[serde(default)]
    pub description: BTreeMap<String, String>,
    pub version: String,
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub loudness_lufs: Option<f32>,
    #[serde(default)]
    pub true_peak_dbtp: Option<f32>,
    #[serde(default)]
    pub art: Option<String>,
    pub beds: Vec<Bed>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub compatibility: Compatibility,
    /// Per-asset or pack-level attribution, required for CC BY content.
    #[serde(default)]
    pub credits: Vec<Credit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    /// Safe relative asset path, or omitted for a pack-level credit.
    #[serde(default)]
    pub asset: Option<String>,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub source: String,
    pub license: String,
    #[serde(default)]
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bed {
    pub id: String,
    pub file: String,
    #[serde(rename = "loop")]
    pub loop_region: LoopRegion,
    #[serde(default)]
    pub gain_db: f32,
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub lfo: Option<Lfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopRegion {
    pub start_sample: u64,
    pub end_sample: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LfoTarget {
    Gain,
    Pan,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lfo {
    pub target: LfoTarget,
    #[serde(default = "default_lfo_depth")]
    pub depth: f32,
    #[serde(default = "default_lfo_period")]
    pub period_sec: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub files: Vec<String>,
    #[serde(default = "default_min_interval")]
    pub min_interval_sec: f32,
    #[serde(default = "default_max_interval")]
    pub max_interval_sec: f32,
    #[serde(default)]
    pub initial_delay_sec: Option<f32>,
    #[serde(default = "default_gain_range")]
    pub gain_db: [f32; 2],
    #[serde(default)]
    pub pan: [f32; 2],
    #[serde(default = "default_fade_in_ms")]
    pub fade_in_ms: u32,
    #[serde(default = "default_fade_out_ms")]
    pub fade_out_ms: u32,
    #[serde(default = "default_max_voices")]
    pub max_voices: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    #[serde(default)]
    pub excludes: Vec<String>,
}

fn default_lfo_depth() -> f32 {
    1.0
}

fn default_lfo_period() -> f32 {
    60.0
}

fn default_min_interval() -> f32 {
    20.0
}

fn default_max_interval() -> f32 {
    60.0
}

fn default_gain_range() -> [f32; 2] {
    [-8.0, -3.0]
}

fn default_fade_in_ms() -> u32 {
    500
}

fn default_fade_out_ms() -> u32 {
    1000
}

fn default_max_voices() -> u8 {
    1
}

impl Manifest {
    pub fn from_json_str(json: &str) -> Result<Self, PackError> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load(dir: &Path) -> Result<Self, PackError> {
        let path = dir.join("manifest.json");
        let text = std::fs::read_to_string(&path)?;
        let manifest: Self = serde_json::from_str(&text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn display_name(&self, language: &str) -> &str {
        self.name
            .get(language)
            .or_else(|| self.name.get("en"))
            .map(String::as_str)
            .unwrap_or(&self.id)
    }

    pub fn description(&self, language: &str) -> &str {
        self.description
            .get(language)
            .or_else(|| self.description.get("en"))
            .map(String::as_str)
            .unwrap_or("")
    }

    /// All audio and art paths referenced by the manifest.
    pub fn asset_paths(&self) -> impl Iterator<Item = &str> {
        self.beds
            .iter()
            .map(|bed| bed.file.as_str())
            .chain(
                self.events
                    .iter()
                    .flat_map(|event| event.files.iter().map(String::as_str)),
            )
            .chain(self.art.as_deref())
    }

    /// Structural validation. Does not touch the filesystem.
    pub fn validate(&self) -> Result<(), PackError> {
        let mut issues = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            issues.push(format!(
                "unsupported schemaVersion {} (expected {SCHEMA_VERSION})",
                self.schema_version
            ));
        }
        if !is_valid_id(&self.id) {
            issues.push(format!(
                "id must be 1..={MAX_ID_LEN} chars of lowercase ASCII letters, digits, or '-' (got {:?})",
                self.id
            ));
        }
        if self
            .name
            .get("en")
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            issues.push("name.en is required".to_string());
        }
        if self.version.trim().is_empty() {
            issues.push("version is required".to_string());
        }
        if self.license.trim().is_empty() {
            issues.push("license is required".to_string());
        }
        if self.beds.is_empty() {
            issues.push("at least one bed is required".to_string());
        }
        if self.beds.len() > 2 {
            issues.push("at most two beds are supported".to_string());
        }
        if self.events.len() > 16 {
            issues.push("at most 16 event types are supported".to_string());
        }

        if let Some(art) = &self.art {
            if !is_safe_relative_path(art) {
                issues.push(format!(
                    "art path must be a safe relative path (got {art:?})"
                ));
            }
        }

        let mut bed_ids = std::collections::BTreeSet::new();
        for bed in &self.beds {
            if bed.id.trim().is_empty() {
                issues.push("bed id is required".to_string());
            }
            if !bed_ids.insert(bed.id.clone()) {
                issues.push(format!("duplicate bed id {:?}", bed.id));
            }
            if !is_safe_relative_path(&bed.file) {
                issues.push(format!(
                    "bed {:?}: file must be a safe relative path (got {:?})",
                    bed.id, bed.file
                ));
            }
            validate_loop(&bed.loop_region, &bed.id, &mut issues);
            if !bed.gain_db.is_finite() || !(-60.0..=6.0).contains(&bed.gain_db) {
                issues.push(format!(
                    "bed {:?}: gainDb must be finite and in -60..=6",
                    bed.id
                ));
            }
            if !bed.pan.is_finite() || !(-1.0..=1.0).contains(&bed.pan) {
                issues.push(format!(
                    "bed {:?}: pan must be finite and in -1..=1",
                    bed.id
                ));
            }
            if let Some(lfo) = &bed.lfo {
                if !lfo.period_sec.is_finite() || !(5.0..=600.0).contains(&lfo.period_sec) {
                    issues.push(format!(
                        "bed {:?}: lfo.periodSec must be in 5..=600",
                        bed.id
                    ));
                }
                let max_depth = match lfo.target {
                    LfoTarget::Gain => 6.0,
                    LfoTarget::Pan => 1.0,
                };
                if !lfo.depth.is_finite() || !(0.0..=max_depth).contains(&lfo.depth) {
                    issues.push(format!(
                        "bed {:?}: lfo.depth must be in 0..={max_depth} for {:?}",
                        bed.id, lfo.target
                    ));
                }
            }
        }

        let mut event_ids = std::collections::BTreeSet::new();
        for event in &self.events {
            if event.id.trim().is_empty() {
                issues.push("event id is required".to_string());
            }
            if !event_ids.insert(event.id.clone()) {
                issues.push(format!("duplicate event id {:?}", event.id));
            }
            if event.files.is_empty() {
                issues.push(format!(
                    "event {:?}: at least one file is required",
                    event.id
                ));
            }
            if event.files.len() > 16 {
                issues.push(format!(
                    "event {:?}: at most 16 files are supported",
                    event.id
                ));
            }
            for file in &event.files {
                if !is_safe_relative_path(file) {
                    issues.push(format!(
                        "event {:?}: file must be a safe relative path (got {file:?})",
                        event.id
                    ));
                }
            }
            if !event.min_interval_sec.is_finite()
                || !(1.0..=3600.0).contains(&event.min_interval_sec)
            {
                issues.push(format!(
                    "event {:?}: minIntervalSec must be in 1..=3600",
                    event.id
                ));
            }
            if event.max_interval_sec < event.min_interval_sec || event.max_interval_sec > 3600.0 {
                issues.push(format!(
                    "event {:?}: maxIntervalSec must be >= minIntervalSec and <= 3600",
                    event.id
                ));
            }
            if let Some(delay) = event.initial_delay_sec {
                if !delay.is_finite() || !(0.0..=3600.0).contains(&delay) {
                    issues.push(format!(
                        "event {:?}: initialDelaySec must be in 0..=3600",
                        event.id
                    ));
                }
            }
            if !event.gain_db[0].is_finite()
                || !event.gain_db[1].is_finite()
                || event.gain_db[0] > event.gain_db[1]
            {
                issues.push(format!(
                    "event {:?}: gainDb must be finite [min, max] with min <= max",
                    event.id
                ));
            }
            if event.gain_db[0] < -60.0 || event.gain_db[1] > 6.0 {
                issues.push(format!("event {:?}: gainDb must be in -60..=6", event.id));
            }
            if !event.pan[0].is_finite()
                || !event.pan[1].is_finite()
                || !(-1.0..=1.0).contains(&event.pan[0])
                || !(-1.0..=1.0).contains(&event.pan[1])
            {
                issues.push(format!(
                    "event {:?}: pan must be finite and in -1..=1",
                    event.id
                ));
            }
            if event.fade_in_ms > 10_000 || event.fade_out_ms > 10_000 {
                issues.push(format!("event {:?}: fades must be <= 10000 ms", event.id));
            }
            if !(1..=4).contains(&event.max_voices) {
                issues.push(format!("event {:?}: maxVoices must be in 1..=4", event.id));
            }
        }

        for excluded in &self.compatibility.excludes {
            if !is_valid_id(excluded) {
                issues.push(format!(
                    "compatibility.excludes: invalid pack id {excluded:?}"
                ));
            }
        }

        if self.credits.len() > 64 {
            issues.push("at most 64 credits are supported".to_string());
        }
        let assets: std::collections::BTreeSet<&str> = self.asset_paths().collect();
        for (index, credit) in self.credits.iter().enumerate() {
            if credit.title.trim().is_empty() {
                issues.push(format!("credit {index}: title is required"));
            }
            if credit.author.trim().is_empty() {
                issues.push(format!("credit {index}: author is required"));
            }
            if credit.license.trim().is_empty() {
                issues.push(format!("credit {index}: license is required"));
            }
            if let Some(asset) = &credit.asset {
                if !is_safe_relative_path(asset) {
                    issues.push(format!(
                        "credit {index}: asset must be a safe relative path (got {asset:?})"
                    ));
                } else if !assets.contains(asset.as_str()) {
                    issues.push(format!(
                        "credit {index}: asset {asset:?} is not referenced by the manifest"
                    ));
                }
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(PackError::Validation(issues.join("; ")))
        }
    }

    /// Structural validation plus existence/safety checks for every asset.
    pub fn validate_files(&self, dir: &Path) -> Result<(), PackError> {
        self.validate()?;
        let mut issues = Vec::new();

        for path in self.asset_paths() {
            let full = dir.join(path);
            if !is_safe_relative_path(path) {
                issues.push(format!("unsafe path {path:?}"));
            } else if !full.is_file() {
                issues.push(format!("missing file {path:?}"));
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(PackError::Validation(issues.join("; ")))
        }
    }

    pub fn asset_path(&self, relative: &str) -> Result<PathBuf, PackError> {
        if !is_safe_relative_path(relative) {
            return Err(PackError::Validation(format!(
                "unsafe relative path: {relative:?}"
            )));
        }
        Ok(PathBuf::from(relative))
    }
}

fn validate_loop(region: &LoopRegion, bed_id: &str, issues: &mut Vec<String>) {
    if region.end_sample <= region.start_sample {
        issues.push(format!(
            "bed {bed_id:?}: loop.endSample must be greater than loop.startSample"
        ));
        return;
    }
    let length = region.end_sample - region.start_sample;
    // The manifest expresses loop points at the source sample rate. 48 kHz is
    // the production rate; allow 1 s .. 10 min here and enforce the real file
    // length in the loader.
    if length < 48_000 {
        issues.push(format!("bed {bed_id:?}: loop must be at least 1 second"));
    }
    if length > 48_000 * 600 {
        issues.push(format!("bed {bed_id:?}: loop must be at most 10 minutes"));
    }
}

pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
}

/// Reject absolute paths, parent traversal, empty segments, and Windows
/// separators. This is used for pack-relative asset paths before joining.
pub fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        return false;
    }
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return false;
    }
    let candidate = Path::new(path);
    candidate.components().all(|component| match component {
        Component::Normal(segment) => segment
            .to_str()
            .map(|segment| !segment.is_empty())
            .unwrap_or(false),
        Component::CurDir => false,
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest_json() -> String {
        serde_json::json!({
            "schemaVersion": 1,
            "id": "test-rain",
            "name": { "en": "Test Rain" },
            "description": { "en": "Synthetic rain for tests." },
            "version": "0.1.0",
            "license": "CC0-1.0",
            "tags": ["water", "test"],
            "loudnessLufs": -23.0,
            "truePeakDbtp": -1.5,
            "art": "art/cover.webp",
            "beds": [{
                "id": "rain",
                "file": "audio/rain.wav",
                "loop": { "startSample": 0, "endSample": 1440000 },
                "gainDb": -3.0,
                "pan": 0.0,
                "lfo": { "target": "gain", "depth": 1.5, "periodSec": 47.0 }
            }],
            "events": [{
                "id": "thunder",
                "files": ["audio/thunder-01.wav"],
                "minIntervalSec": 25.0,
                "maxIntervalSec": 90.0,
                "gainDb": [-10.0, -4.0],
                "pan": [-0.5, 0.5],
                "fadeInMs": 800,
                "fadeOutMs": 1200,
                "maxVoices": 1
            }]
        })
        .to_string()
    }

    #[test]
    fn parses_and_validates_manifest() {
        let manifest = Manifest::from_json_str(&valid_manifest_json()).unwrap();
        assert_eq!(manifest.id, "test-rain");
        assert_eq!(manifest.display_name("en"), "Test Rain");
        assert_eq!(manifest.beds.len(), 1);
        assert_eq!(manifest.events.len(), 1);
        assert_eq!(manifest.events[0].gain_db, [-10.0, -4.0]);
    }

    #[test]
    fn rejects_missing_english_name() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value["name"] = serde_json::json!({ "it": "Pioggia" });
        let result = Manifest::from_json_str(&value.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_bad_loop() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value["beds"][0]["loop"] = serde_json::json!({ "startSample": 100, "endSample": 50 });
        let result = Manifest::from_json_str(&value.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(!is_safe_relative_path("../secret"));
        assert!(!is_safe_relative_path("/etc/passwd"));
        assert!(!is_safe_relative_path("audio\\rain.wav"));
        assert!(is_safe_relative_path("audio/rain.wav"));
        assert!(is_safe_relative_path("cover.webp"));
    }

    #[test]
    fn rejects_duplicate_bed_ids() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        let mut second = value["beds"][0].clone();
        second["file"] = serde_json::json!("audio/rain2.wav");
        value["beds"].as_array_mut().unwrap().push(second);
        assert!(Manifest::from_json_str(&value.to_string()).is_err());
    }

    #[test]
    fn validate_files_checks_assets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("audio")).unwrap();
        std::fs::create_dir_all(dir.path().join("art")).unwrap();
        std::fs::write(dir.path().join("audio/rain.wav"), b"x").unwrap();
        std::fs::write(dir.path().join("audio/thunder-01.wav"), b"x").unwrap();
        std::fs::write(dir.path().join("art/cover.webp"), b"x").unwrap();
        std::fs::write(dir.path().join("manifest.json"), valid_manifest_json()).unwrap();

        let manifest = Manifest::load(dir.path()).unwrap();
        manifest.validate_files(dir.path()).unwrap();

        std::fs::remove_file(dir.path().join("audio/rain.wav")).unwrap();
        assert!(manifest.validate_files(dir.path()).is_err());
    }

    #[test]
    fn parses_and_validates_credits() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value["credits"] = serde_json::json!([
            {
                "asset": "audio/rain.wav",
                "title": "Rain",
                "author": "Tester",
                "source": "https://example.com/rain",
                "license": "CC0-1.0",
                "modified": true
            },
            {
                "title": "Oniris",
                "author": "Oniris",
                "license": "CC0-1.0"
            }
        ]);

        let manifest = Manifest::from_json_str(&value.to_string()).unwrap();
        assert_eq!(manifest.credits.len(), 2);
        assert_eq!(manifest.credits[0].asset.as_deref(), Some("audio/rain.wav"));
        assert!(manifest.credits[0].modified);
        assert!(manifest.credits[1].asset.is_none());
    }

    #[test]
    fn rejects_bad_credits() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_manifest_json()).unwrap();
        value["credits"] = serde_json::json!([
            {
                "asset": "../evil.wav",
                "title": "Unsafe",
                "author": "Tester",
                "license": "CC0-1.0"
            },
            {
                "asset": "audio/missing.wav",
                "title": "Missing",
                "author": "Tester",
                "license": "CC0-1.0"
            },
            {
                "title": "",
                "author": "",
                "license": ""
            }
        ]);

        let error = Manifest::from_json_str(&value.to_string()).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("safe relative path"));
        assert!(message.contains("not referenced"));
        assert!(message.contains("title is required"));
        assert!(message.contains("author is required"));
        assert!(message.contains("license is required"));
    }
}
