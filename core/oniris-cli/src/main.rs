mod generate;
mod loudness;
mod opus_encode;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use generate::AudioFormat;
use hound::{WavReader, WavSpec, WavWriter};
use oniris_core::{decode_audio, Engine, MixEntry, MixSpec, DEFAULT_BLOCK_FRAMES};
use oniris_pack::{pack_dir, unpack_archive, verify_archive, Manifest};

#[derive(Debug, Parser)]
#[command(name = "oniris", version, about = "Oniris soundscape core tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Render a mix to a stereo WAV file.
    Render {
        /// Pack directory. Repeat for each pack in the mix.
        #[arg(long = "pack", required = true)]
        packs: Vec<PathBuf>,
        /// Duration in minutes.
        #[arg(long, default_value_t = 10.0)]
        minutes: f32,
        /// Output WAV path.
        #[arg(long, default_value = "oniris-render.wav")]
        out: PathBuf,
        /// Deterministic RNG seed.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Fade-in duration in seconds.
        #[arg(long, default_value_t = 2.0)]
        fade_in: f32,
        /// Optional sleep timer in minutes; fades over the final 60 seconds.
        #[arg(long, default_value_t = 0.0)]
        timer_minutes: f32,
    },
    /// Validate a pack directory and decode every asset.
    Validate { path: PathBuf },
    /// Print basic information about a WAV render.
    Analyze {
        path: PathBuf,
        /// Single-sample jump treated as a click, as a fraction of full scale.
        #[arg(long, default_value_t = 0.25)]
        click_threshold: f32,
    },
    /// Encode a WAV or Opus file as a 48 kHz Ogg/Opus asset.
    EncodeOpus {
        input: PathBuf,
        output: PathBuf,
        /// Target bitrate in bits per second.
        #[arg(long, default_value_t = 96_000)]
        bitrate: i32,
    },
    /// Measure integrated loudness (LUFS) and true peak (dBTP).
    Loudness { path: PathBuf },
    /// Normalize a file to a loudness target while respecting true peak.
    Normalize {
        input: PathBuf,
        /// Output path; `.wav`, `.opus`, and `.ogg` are supported.
        output: PathBuf,
        /// Integrated loudness target in LUFS.
        #[arg(long, default_value_t = -23.0, allow_hyphen_values = true)]
        target_lufs: f32,
        /// True-peak ceiling in dBTP.
        #[arg(long, default_value_t = -1.5, allow_hyphen_values = true)]
        true_peak: f32,
        /// Opus bitrate for `.opus`/`.ogg` output.
        #[arg(long, default_value_t = 96_000)]
        bitrate: i32,
    },
    /// Generate synthetic test packs (rain + fire) for development.
    GenTestPacks {
        out: PathBuf,
        /// Audio format for the generated assets. Opus is the production format;
        /// WAV remains available as a lossless debugging fallback.
        #[arg(long, value_enum, default_value = "opus")]
        format: AudioFormat,
    },
    /// Package a pack directory as a deterministic `.onirispack` archive.
    Pack {
        /// Pack directory containing `manifest.json`.
        dir: PathBuf,
        /// Output `.onirispack` path.
        out: PathBuf,
    },
    /// Validate a `.onirispack` archive without extracting it.
    VerifyPack {
        archive: PathBuf,
        /// Expected lowercase hex SHA-256 of the archive.
        #[arg(long)]
        sha256: Option<String>,
    },
    /// Validate and extract a `.onirispack` into a new or empty directory.
    Unpack {
        archive: PathBuf,
        /// Destination directory.
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render {
            packs,
            minutes,
            out,
            seed,
            fade_in,
            timer_minutes,
        } => render(&packs, minutes, &out, seed, fade_in, timer_minutes),
        Command::Validate { path } => validate(&path),
        Command::Analyze {
            path,
            click_threshold,
        } => analyze(&path, click_threshold),
        Command::EncodeOpus {
            input,
            output,
            bitrate,
        } => encode_opus_file(&input, &output, bitrate),
        Command::Loudness { path } => measure_loudness(&path),
        Command::Normalize {
            input,
            output,
            target_lufs,
            true_peak,
            bitrate,
        } => normalize_file(&input, &output, target_lufs, true_peak, bitrate),
        Command::GenTestPacks { out, format } => generate::generate_test_packs(&out, format),
        Command::Pack { dir, out } => pack_archive(&dir, &out),
        Command::VerifyPack { archive, sha256 } => verify_pack(&archive, sha256.as_deref()),
        Command::Unpack { archive, dir } => unpack_pack(&archive, &dir),
    }
}

fn pack_archive(dir: &Path, out: &Path) -> Result<()> {
    let info =
        pack_dir(dir, out).with_context(|| format!("failed to package {}", dir.display()))?;
    println!(
        "packed {} -> {} ({:.1} kB, {} files, sha256 {})",
        info.manifest.id,
        out.display(),
        info.bytes as f32 / 1024.0,
        info.files,
        info.sha256,
    );
    Ok(())
}

fn verify_pack(archive: &Path, expected: Option<&str>) -> Result<()> {
    let info = verify_archive(archive)
        .with_context(|| format!("failed to verify {}", archive.display()))?;
    if let Some(expected) = expected {
        if !info.sha256.eq_ignore_ascii_case(expected) {
            bail!("SHA-256 mismatch: expected {expected}, got {}", info.sha256);
        }
    }
    println!(
        "OK {} ({:.1} kB, {} files, sha256 {})",
        info.manifest.id,
        info.bytes as f32 / 1024.0,
        info.files,
        info.sha256,
    );
    Ok(())
}

fn unpack_pack(archive: &Path, dir: &Path) -> Result<()> {
    let info = unpack_archive(archive, dir)
        .with_context(|| format!("failed to unpack {}", archive.display()))?;
    println!(
        "unpacked {} into {} ({} files, sha256 {})",
        info.manifest.id,
        dir.display(),
        info.files,
        info.sha256,
    );
    Ok(())
}

fn encode_opus_file(input: &Path, output: &Path, bitrate: i32) -> Result<()> {
    let audio = decode_audio(input, 48_000)
        .with_context(|| format!("failed to decode {}", input.display()))?;
    let stats = opus_encode::encode_opus(
        output,
        &audio.samples,
        audio.channels,
        audio.sample_rate,
        bitrate,
    )?;
    let bytes = std::fs::metadata(output)
        .with_context(|| format!("failed to inspect {}", output.display()))?
        .len();
    println!(
        "encoded {} -> {} ({:.2} s, {} packets, {:.1} kB)",
        input.display(),
        output.display(),
        stats.frames as f32 / audio.sample_rate as f32,
        stats.packet_count,
        bytes as f32 / 1024.0,
    );
    Ok(())
}

fn measure_loudness(path: &Path) -> Result<()> {
    let audio = decode_audio(path, 48_000)
        .with_context(|| format!("failed to decode {}", path.display()))?;
    let measurement = loudness::measure(&audio.samples, audio.channels, 48_000);
    println!(
        "{}: integrated {} LUFS, true peak {} dBTP, sample peak {} dBFS, {} gated blocks",
        path.display(),
        format_level(measurement.integrated_lufs),
        format_level(measurement.true_peak_dbtp),
        format_level(measurement.sample_peak_dbfs),
        measurement.gated_blocks,
    );
    Ok(())
}

fn normalize_file(
    input: &Path,
    output: &Path,
    target_lufs: f32,
    true_peak: f32,
    bitrate: i32,
) -> Result<()> {
    if !target_lufs.is_finite() || !true_peak.is_finite() {
        bail!("--target-lufs and --true-peak must be finite");
    }

    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .context("output path needs a .wav, .opus, or .ogg extension")?;
    if !matches!(extension.as_str(), "wav" | "opus" | "ogg") {
        bail!("unsupported output format .{extension}; use .wav, .opus, or .ogg");
    }

    let audio = decode_audio(input, 48_000)
        .with_context(|| format!("failed to decode {}", input.display()))?;
    let before = loudness::measure(&audio.samples, audio.channels, 48_000);
    if !before.integrated_lufs.is_finite() {
        bail!("cannot normalize a silent signal");
    }

    // Reach the loudness target unless that would push the true peak over the
    // ceiling; in that case keep the peak and accept a quieter result.
    let loudness_gain = target_lufs - before.integrated_lufs;
    let peak_gain = true_peak - before.true_peak_dbtp;
    let gain_db = loudness_gain.min(peak_gain);
    if !gain_db.is_finite() {
        bail!("could not compute a normalization gain");
    }

    let normalized = loudness::apply_gain(&audio.samples, gain_db);

    match extension.as_str() {
        "wav" => write_wav_48k(output, audio.channels, &normalized)?,
        "opus" | "ogg" => {
            opus_encode::encode_opus(output, &normalized, audio.channels, 48_000, bitrate)?;
        }
        _ => unreachable!(),
    }

    // Re-measure the written file: lossy encoding can move both loudness and
    // true peak slightly, so report what actually landed on disk.
    let written = decode_audio(output, 48_000)
        .with_context(|| format!("failed to verify {}", output.display()))?;
    let output_measurement = loudness::measure(&written.samples, written.channels, 48_000);
    if output_measurement.true_peak_dbtp > true_peak + 0.3 {
        eprintln!(
            "warning: encoded true peak is {} dBTP, above the {} dBTP target",
            format_level(output_measurement.true_peak_dbtp),
            format_level(true_peak),
        );
    }

    println!(
        "normalized {} -> {}: {} -> {} LUFS, {} -> {} dBTP, applied {:+.2} dB",
        input.display(),
        output.display(),
        format_level(before.integrated_lufs),
        format_level(output_measurement.integrated_lufs),
        format_level(before.true_peak_dbtp),
        format_level(output_measurement.true_peak_dbtp),
        gain_db,
    );
    Ok(())
}

fn write_wav_48k(path: &Path, channels: u16, samples: &[i16]) -> Result<()> {
    let spec = WavSpec {
        channels,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)
        .with_context(|| format!("failed to create {}", path.display()))?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn format_level(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.1}")
    } else {
        "-inf".to_string()
    }
}

fn render(
    packs: &[PathBuf],
    minutes: f32,
    out: &Path,
    seed: u64,
    fade_in: f32,
    timer_minutes: f32,
) -> Result<()> {
    if minutes <= 0.0 {
        anyhow::bail!("--minutes must be positive");
    }

    let mut engine = Engine::with_seed(48_000, seed);
    engine.set_fade_in(Duration::from_secs_f32(fade_in.max(0.0)));

    let mut entries = Vec::with_capacity(packs.len());
    for pack in packs {
        let id = engine
            .load_pack(pack)
            .with_context(|| format!("failed to load {}", pack.display()))?;
        println!("loaded {id} from {}", pack.display());
        entries.push(MixEntry {
            id,
            gain_db: 0.0,
            muted: false,
        });
    }

    engine
        .set_mix(&MixSpec::new(entries))
        .context("failed to apply mix")?;

    if timer_minutes > 0.0 {
        engine.set_sleep_timer(
            Duration::from_secs_f32(timer_minutes * 60.0),
            Duration::from_secs(60),
        );
    }

    engine.play();

    let spec = WavSpec {
        channels: 2,
        sample_rate: engine.sample_rate(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(out, spec)
        .with_context(|| format!("failed to create {}", out.display()))?;

    let total_frames = (minutes * 60.0 * engine.sample_rate() as f32).round() as usize;
    let mut written = 0usize;
    let mut peak = 0.0f32;
    let mut block = vec![0i16; DEFAULT_BLOCK_FRAMES * 2];

    while written < total_frames {
        let frames = (total_frames - written).min(DEFAULT_BLOCK_FRAMES);
        let stats = engine.render_i16(&mut block, frames);
        for sample in &block[..frames * 2] {
            writer.write_sample(*sample)?;
        }
        written += frames;
        peak = peak.max(stats.peak);
        if stats.finished {
            println!(
                "sleep timer finished after {:.1} s",
                written as f32 / 48_000.0
            );
            break;
        }
    }

    writer.finalize().context("failed to finalize WAV")?;

    println!(
        "wrote {} ({:.1} s, peak {:.1} dBFS, decoded {:.1} MB)",
        out.display(),
        written as f32 / engine.sample_rate() as f32,
        linear_to_db(peak),
        engine.decoded_bytes() as f32 / (1024.0 * 1024.0),
    );
    Ok(())
}

fn validate(path: &Path) -> Result<()> {
    let manifest = Manifest::load(path)
        .with_context(|| format!("failed to load manifest from {}", path.display()))?;
    manifest
        .validate_files(path)
        .with_context(|| format!("pack validation failed for {}", path.display()))?;

    let mut engine = Engine::new(48_000);
    let id = engine
        .load_pack(path)
        .with_context(|| format!("failed to decode pack {}", path.display()))?;
    let pack = engine.loaded_pack(&id).expect("just loaded");

    println!("OK {}", manifest.display_name("en"));
    println!("  id:        {id}");
    println!("  version:   {}", manifest.version);
    println!("  license:   {}", manifest.license);
    println!("  beds:      {}", pack.bed_count());
    println!("  events:    {}", pack.event_count());
    println!(
        "  decoded:   {:.2} MB",
        pack.decoded_bytes() as f32 / (1024.0 * 1024.0)
    );
    Ok(())
}

fn analyze(path: &Path, click_threshold: f32) -> Result<()> {
    let mut reader =
        WavReader::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;

    let mut peak = 0i32;
    let mut sum_squares = 0f64;
    let mut sample_count = 0usize;
    let mut previous = vec![0i32; channels];
    let threshold = (click_threshold.clamp(0.0, 1.0) * i16::MAX as f32) as i32;
    let mut max_delta = 0i32;
    let mut clicks = 0usize;

    for sample in reader.samples::<i16>() {
        let value = i32::from(sample.context("failed to read samples")?);
        let channel = sample_count % channels;
        if sample_count >= channels {
            let delta = (value - previous[channel]).abs();
            max_delta = max_delta.max(delta);
            if delta > threshold {
                clicks += 1;
            }
        }
        previous[channel] = value;
        peak = peak.max(value.abs());
        sum_squares += (value as f64) * (value as f64);
        sample_count += 1;
    }

    if sample_count == 0 {
        anyhow::bail!("file contains no samples");
    }

    let frames = sample_count / channels;
    let rms = (sum_squares / sample_count as f64).sqrt();

    println!("{}", path.display());
    println!("  sample rate: {}", spec.sample_rate);
    println!("  channels:    {}", spec.channels);
    println!("  frames:      {frames}");
    println!(
        "  duration:    {:.2} s",
        frames as f32 / spec.sample_rate as f32
    );
    println!(
        "  peak:        {:.2} dBFS",
        linear_to_db(peak as f32 / i16::MAX as f32)
    );
    println!(
        "  rms:         {:.2} dBFS",
        linear_to_db(rms as f32 / i16::MAX as f32)
    );
    println!(
        "  max delta:   {} ({:.1} dBFS)",
        max_delta,
        linear_to_db(max_delta as f32 / i16::MAX as f32)
    );
    println!(
        "  jumps:       {clicks} above {:.0}% full scale",
        click_threshold * 100.0
    );
    Ok(())
}

fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * linear.log10()
    }
}
