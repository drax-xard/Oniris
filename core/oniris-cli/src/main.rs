mod generate;
mod opus_encode;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use generate::AudioFormat;
use hound::{WavReader, WavSpec, WavWriter};
use oniris_core::{decode_audio, Engine, MixEntry, MixSpec, DEFAULT_BLOCK_FRAMES};
use oniris_pack::Manifest;

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
    Analyze { path: PathBuf },
    /// Encode a WAV or Opus file as a 48 kHz Ogg/Opus asset.
    EncodeOpus {
        input: PathBuf,
        output: PathBuf,
        /// Target bitrate in bits per second.
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
        Command::Analyze { path } => analyze(&path),
        Command::EncodeOpus {
            input,
            output,
            bitrate,
        } => encode_opus_file(&input, &output, bitrate),
        Command::GenTestPacks { out, format } => generate::generate_test_packs(&out, format),
    }
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

fn analyze(path: &Path) -> Result<()> {
    let mut reader =
        WavReader::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<_, _>>()
        .context("failed to read samples")?;

    if samples.is_empty() {
        anyhow::bail!("file contains no samples");
    }

    let channels = spec.channels as usize;
    let frames = samples.len() / channels;
    let mut peak = 0i32;
    let mut sum_squares = 0f64;
    for sample in &samples {
        let value = *sample as i32;
        peak = peak.max(value.abs());
        sum_squares += (value as f64) * (value as f64);
    }
    let rms = (sum_squares / samples.len() as f64).sqrt();

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
    Ok(())
}

fn linear_to_db(linear: f32) -> f32 {
    if linear <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * linear.log10()
    }
}
