#!/usr/bin/env python3
"""Turn source recordings into a validated, packaged Oniris sound pack.

The pipeline is driven by a JSON pack spec (see `specs/`). For every bed and
event it does:

  decode -> trim -> cleanup (DC / rumble / optional hum) -> loop with an
  equal-power tail-into-head crossfade -> loudness normalisation through the
  Rust CLI (target LUFS with a true-peak ceiling) -> audio QA gate ->
  gapless Opus encode -> manifest + LICENSE + README -> `oniris validate` ->
  `oniris pack` -> archive SHA-256.

Masters never enter the repository: specs point at absolute paths outside the
repo (e.g. ~/oniris-sources), and generated packs land under dist/ (ignored).

Requires: python3 + numpy, ffmpeg, and a built oniris-cli.

Usage:
    tools/asset-pipeline/ingest.py build specs/seaside.json
    tools/asset-pipeline/ingest.py build specs/seaside.json --skip-package
    tools/asset-pipeline/ingest.py build specs/seaside.json --only sea
    tools/asset-pipeline/ingest.py probe ~/oniris-sources/masters/*
"""

import argparse
import csv
import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[2]
SAMPLE_RATE = 48_000
CLAIMS = {
    "CC0-1.0": "https://creativecommons.org/publicdomain/zero/1.0/",
    "CC-BY-4.0": "https://creativecommons.org/licenses/by/4.0/",
    "CC-BY-3.0": "https://creativecommons.org/licenses/by/3.0/",
    "Public Domain": "https://www.nps.gov/subjects/sound/index.htm",
}
LICENSE_RANK = {"CC0-1.0": 0, "Public Domain": 0, "CC-BY-3.0": 1, "CC-BY-4.0": 2}


# ---------------------------------------------------------------- subprocess


def run(command, capture=True, input_bytes=None, binary=False):
    result = subprocess.run(
        [str(part) for part in command],
        capture_output=capture,
        input=input_bytes,
        text=None if binary else (input_bytes is None),
        check=False,
    )
    if result.returncode != 0:
        stderr = result.stderr if capture else ""
        raise SystemExit(f"command failed ({result.returncode}): {' '.join(map(str, command))}\n{stderr}")
    return result


def oniris_cli(*args, capture=True):
    """Prefer a built binary; fall back to `cargo run`."""
    for profile in ("release", "debug"):
        for name in ("oniris", "oniris-cli"):
            binary = REPO / "core" / "target" / profile / name
            if binary.exists():
                return run([binary, *args], capture=capture)
    return run(
        ["cargo", "run", "--release", "--manifest-path", REPO / "core" / "Cargo.toml",
         "-p", "oniris-cli", "--", *args],
        capture=capture,
    )


# --------------------------------------------------------------------- audio


def probe(path):
    out = run([
        "ffprobe", "-v", "error", "-select_streams", "a:0",
        "-show_entries", "stream=codec_name,sample_rate,channels,bits_per_sample,duration",
        "-of", "json", str(path),
    ]).stdout
    stream = json.loads(out)["streams"][0]
    bits = stream.get("bits_per_sample") or 0
    codec = stream.get("codec_name", "")
    if codec.startswith("pcm_f"):
        depth = f"{bits} float"
    elif codec.startswith("pcm_s"):
        depth = str(bits)
    else:
        depth = codec
    return {
        "sample_rate": int(stream["sample_rate"]),
        "channels": int(stream["channels"]),
        "bit_depth": depth,
        "duration_sec": round(float(stream.get("duration", 0.0)), 3),
        "codec": codec,
    }


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def decode(path, channels, trim=None, filters=None):
    """Decode to float32 numpy (frames x channels) at 48 kHz.

    `filters` is an ffmpeg filter graph string applied before the sample-rate
    conversion, so cleanup happens at the source rate.
    """
    command = ["ffmpeg", "-v", "error", "-i", str(path)]
    if trim:
        command += ["-ss", f"{trim[0]}", "-t", f"{trim[1] - trim[0]}"]
    command += ["-map", "0:a:0"]
    if filters:
        command += ["-af", filters]
    command += ["-ac", str(channels), "-ar", str(SAMPLE_RATE),
                "-f", "f32le", "-acodec", "pcm_f32le", "-"]
    raw = run(command, binary=True).stdout
    audio = np.frombuffer(raw, dtype="<f4").astype(np.float64).reshape(-1, channels)
    return audio


def load_float(path, channels=2):
    """Decode any audio file to float64 at 48 kHz."""
    raw = run(["ffmpeg", "-v", "error", "-i", path, "-map", "0:a:0",
               "-ac", str(channels), "-ar", str(SAMPLE_RATE),
               "-f", "f32le", "-acodec", "pcm_f32le", "-"], binary=True).stdout
    return np.frombuffer(raw, dtype="<f4").astype(np.float64).reshape(-1, channels)


def write_wav(path, audio, bit_depth="f32"):
    """Write float audio as a 48 kHz WAV.

    The pipe is always float: feeding raw s16 bytes into a stream declared as
    f32le makes ffmpeg read half as many samples, so the file comes out at half
    the requested duration.
    """
    channels = audio.shape[1]
    codec = "pcm_f32le" if bit_depth == "f32" else "pcm_s16le"
    data = audio.astype("<f4").tobytes()
    run(["ffmpeg", "-v", "error", "-y", "-f", "f32le", "-ar", str(SAMPLE_RATE),
         "-ac", str(channels), "-i", "-", "-c:a", codec, str(path)], input_bytes=data)


def band_energy(audio, bands=None):
    mono = audio.mean(axis=1)
    win = 8192 if len(mono) >= 8192 else 1024
    hop = win // 2
    window = np.hanning(win)
    freqs = np.fft.rfftfreq(win, 1 / SAMPLE_RATE)
    psd = np.zeros(len(freqs))
    for index in range(0, len(mono) - win, hop):
        magnitude = np.abs(np.fft.rfft(mono[index:index + win] * window))
        psd += magnitude ** 2
    total = psd.sum() or 1.0
    psd /= total
    bands = bands or [(0, 60), (60, 120), (120, 300), (300, 1000),
                      (1000, 3000), (3000, 8000), (8000, 20000)]
    return {
        f"{low}-{high}": round(100 * float(psd[(freqs >= low) & (freqs < high)].sum()), 1)
        for low, high in bands
    }


def cosine_seam(audio, window=4096):
    """Spectral continuity of the loop point: 1.0 means indistinguishable."""
    if len(audio) < 2 * window:
        return None
    head = np.abs(np.fft.rfft(audio[:window, 0] * np.hanning(window)))
    tail = np.abs(np.fft.rfft(audio[-window:, 0] * np.hanning(window)))
    denominator = float(np.linalg.norm(head) * np.linalg.norm(tail))
    return None if denominator == 0 else round(float(head @ tail / denominator), 4)


def apply_fades(audio, fade_in_sec, fade_out_sec):
    out = audio.copy()
    ramp_in = int(round(fade_in_sec * SAMPLE_RATE))
    ramp_out = int(round(fade_out_sec * SAMPLE_RATE))
    if ramp_in > 0 and ramp_in <= len(out):
        curve = 0.5 - 0.5 * np.cos(np.linspace(0, np.pi, ramp_in))[:, None]
        out[:ramp_in] *= curve
    if ramp_out > 0 and ramp_out <= len(out):
        curve = 0.5 - 0.5 * np.cos(np.linspace(np.pi, 0, ramp_out))[:, None]
        out[-ramp_out:] *= curve
    return out


def loop_region(audio, length_sec, crossfade_sec, start_sec=0.0):
    """Build an equal-power crossfaded loop of exactly `length_sec` seconds.

    The tail of the region is folded back over its head, so playing the result
    end-to-end and jumping from the last sample to the first is seamless.
    """
    length = int(round(length_sec * SAMPLE_RATE))
    fade = int(round(crossfade_sec * SAMPLE_RATE))
    start = int(round(start_sec * SAMPLE_RATE))
    needed = start + length + fade
    if fade <= 0:
        raise SystemExit("crossfade must be positive")
    if needed > len(audio):
        raise SystemExit(
            f"loop needs {needed / SAMPLE_RATE:.2f} s of source "
            f"(start {start_sec}s + length {length_sec}s + crossfade {crossfade_sec}s) "
            f"but only {len(audio) / SAMPLE_RATE:.2f} s is available",
        )

    out = audio[start:start + length].copy()
    # Equal-power (sin/cos) fade: the folded tail enters as the head leaves, so
    # uncorrelated ambience keeps a constant perceived level.
    t = np.linspace(0.0, np.pi / 2, fade)[:, None]
    head = out[:fade]
    tail = audio[start + length:start + length + fade]
    out[:fade] = head * np.sin(t) + tail * np.cos(t)
    return out, {
        "crossfade_sec": crossfade_sec,
        "source_start_sec": start_sec,
        "consumed_source_sec": round(needed / SAMPLE_RATE, 3),
    }


def running_max(values, window):
    """Sliding maximum (van Herk / Gil-Werman), O(n)."""
    window = max(int(window), 1)
    n = len(values)
    pad = (-n) % window
    padded = np.pad(values, (0, pad), constant_values=-np.inf) if pad else values
    blocks = padded.reshape(-1, window)
    forward = np.maximum.accumulate(blocks, axis=1).ravel()
    backward = np.maximum.accumulate(blocks[:, ::-1], axis=1)[:, ::-1].ravel()
    index = np.arange(n)
    return np.maximum(backward[index], forward[np.minimum(index + window - 1, len(forward) - 1)])


def limit_transients(audio, peak_db, attack_ms=5.0, release_ms=30.0):
    """Pull down isolated transients with a smooth gain envelope.

    Field beds are often mostly quiet with a few close-mic rustles 20 dB above
    the bed. Left alone they set the true-peak ceiling, so the whole bed has to
    be normalised quieter, and the surviving snaps still read as crackle on a
    phone speaker. This caps the envelope at `peak_db` and smooths the gain
    change over `release_ms`, which is inaudible on ambience and leaves the
    steady-state level untouched.

    The detector is a sliding *maximum*, not a moving average: an isolated
    single-sample snap disappears inside a 3 ms mean, so the limiter would never
    engage on exactly the events it exists for.
    """
    attack = max(int(attack_ms / 1000 * SAMPLE_RATE), 1)
    envelope = running_max(np.abs(audio).max(axis=1), attack)

    threshold = 10 ** (peak_db / 20)
    gain = np.minimum(1.0, threshold / np.maximum(envelope, 1e-12))
    release = max(int(release_ms / 1000 * SAMPLE_RATE), 1)
    gain = np.convolve(gain, np.ones(release) / release, mode="same")
    # The smoothed gain can dip after the transient ends; clamping keeps the
    # envelope attack from being softened into the event.
    gain = np.minimum(gain, 1.0)
    return audio * gain[:, None]


def crest_db(audio):
    peak = float(np.abs(audio).max())
    rms = float(np.sqrt((audio.mean(axis=1) ** 2).mean()))
    return round(20 * np.log10(max(peak, 1e-12) / max(rms, 1e-12)), 2)


# --------------------------------------------------------------- processing


def normalize_to(input_path, output_path, target_lufs, true_peak):
    summary = oniris_cli("normalize", input_path, output_path,
                         "--target-lufs", f"{target_lufs}",
                         "--true-peak", f"{true_peak}").stdout.strip()
    return summary


def measure(path):
    result = run([sys.executable, REPO / "tools" / "audio-qa" / "measure.py",
                  "--json", str(path)]).stdout
    return json.loads(result)[0]


def gate(path):
    result = subprocess.run(
        [sys.executable, str(REPO / "tools" / "audio-qa" / "measure.py"),
         "--check", str(path)], capture_output=True, text=True, check=False,
    )
    if result.returncode != 0:
        raise SystemExit(f"audio QA gate failed for {path}\n{result.stdout}{result.stderr}")
    return result.stdout.strip().splitlines()[-1]


def cleanup_filters(highpass_hz=None, hum_hz=None, lowpass_hz=None, eq=None):
    parts = []
    if highpass_hz:
        # Removes DC and sub-rumble; 12 dB/oct keeps the low bed intact.
        parts.append(f"highpass=f={highpass_hz}:poles=2")
    if hum_hz:
        for harmonic in (1, 2, 3, 4):
            parts.append(f"bandreject=f={hum_hz * harmonic}:width_type=h:w=2")
    if lowpass_hz:
        parts.append(f"lowpass=f={lowpass_hz}:poles=2")
    # Tonal shaping: [{"filter": "treble", "f": 4500, "g": -3}, ...] uses ffmpeg's
    # bass/treble shelves, which is how a bright field recording is pulled back
    # toward a sleep-friendly balance without inventing content.
    for shelf in eq or []:
        parts.append(f"{shelf['filter']}=f={shelf['f']}:g={shelf['g']}:width_type=q:w={shelf.get('w', 0.7)}")
    return ",".join(parts) or None


def ingest_bed(bed, work, pack_dir, target_lufs, true_peak):
    source = Path(os.path.expanduser(bed["source"]))
    if not source.exists():
        raise SystemExit(f"bed {bed['id']}: source missing: {source}")

    trim = bed.get("trim")
    trim = (trim["startSec"], trim["startSec"] + trim["lengthSec"]) if trim else None
    filters = cleanup_filters(bed.get("highpassHz", 20), bed.get("humHz"), bed.get("lowpassHz"),
                              bed.get("eq"))
    audio = decode(source, 2, trim=trim, filters=filters)

    dc = [round(float(channel.mean()), 6) for channel in audio.T]
    audio = audio - audio.mean(axis=0)  # residual DC after the high-pass

    loop_spec = bed["loop"]
    looped, loop_info = loop_region(
        audio,
        loop_spec["lengthSec"],
        loop_spec.get("crossfadeSec", 2.0),
        loop_spec.get("startSec", 0.0),
    )

    seam_samples = float(abs(looped[0, 0] - looped[-1, 0]))
    step = max(float(np.percentile(np.abs(np.diff(looped[:, 0])), 99.9)), 1e-9)
    raw_path = work / f"{bed['id']}-loop.wav"
    write_wav(raw_path, looped)

    norm_path = work / f"{bed['id']}-norm.wav"
    normalize_to(raw_path, norm_path, target_lufs, true_peak)

    # Optional transient control: limit the bed, then normalise again so the
    # headroom freed by the limiter goes back into the steady-state level.
    limiter = bed.get("limiter")
    if limiter:
        limited = limit_transients(
            load_float(norm_path),
            limiter["peakDb"],
            limiter.get("releaseMs", 30.0),
        )
        limited_path = work / f"{bed['id']}-limited.wav"
        write_wav(limited_path, limited, bit_depth="s16")
        normalize_to(limited_path, norm_path, target_lufs, true_peak)
        print(f"   limiter {limiter['peakDb']} dBFS: crest "
              f"{crest_db(limited)} dB before re-normalising", flush=True)

    gate_result = gate(norm_path)
    stats = measure(norm_path)

    opus_path = pack_dir / "audio" / f"{bed['id']}.opus"
    opus_path.parent.mkdir(parents=True, exist_ok=True)
    encode = oniris_cli("encode-opus", norm_path, opus_path, "--bitrate", "96000").stdout.strip()

    return {
        "id": bed["id"],
        "source": source,
        "source_info": probe(source),
        "trimmed_sec": round(len(audio) / SAMPLE_RATE, 3),
        "loop_sec": round(len(looped) / SAMPLE_RATE, 3),
        "loop_frames": len(looped),
        **loop_info,
        "dc_offset_before": dc,
        "seam_delta": round(seam_samples, 6),
        "seam_vs_p99": round(seam_samples / step, 2),
        "seam_spectral": cosine_seam(looped),
        "stats": stats,
        "gate": gate_result,
        "encode": encode,
        "opus": opus_path,
        "opus_bytes": opus_path.stat().st_size,
        "lfo": bed.get("lfo"),
        "gain_db": bed.get("gainDb", 0.0),
        "pan": bed.get("pan", 0.0),
        "credit": bed["credit"],
    }


def ingest_event(event, work, pack_dir, target_lufs, true_peak):
    results = []
    for file_spec in event["files"]:
        source = Path(os.path.expanduser(file_spec["source"]))
        if not source.exists():
            raise SystemExit(f"event {event['id']}: source missing: {source}")
        trim = None
        if file_spec.get("trim"):
            trim = (file_spec["trim"]["startSec"],
                    file_spec["trim"]["startSec"] + file_spec["trim"]["lengthSec"])
        filters = cleanup_filters(
            file_spec.get("highpassHz", 60), file_spec.get("humHz"), file_spec.get("lowpassHz"),
            file_spec.get("eq"))
        audio = decode(source, 1, trim=trim, filters=filters)
        audio = apply_fades(audio,
                            file_spec.get("fadeInSec", 0.01),
                            file_spec.get("fadeOutSec", 0.15))

        raw_path = work / f"{Path(file_spec['asset']).stem}.wav"
        write_wav(raw_path, audio)
        norm_path = work / f"{Path(file_spec['asset']).stem}-norm.wav"
        normalize_to(raw_path, norm_path, target_lufs, true_peak)
        gate_result = gate(norm_path)

        opus_path = pack_dir / file_spec["asset"]
        opus_path.parent.mkdir(parents=True, exist_ok=True)
        encode = oniris_cli("encode-opus", norm_path, opus_path,
                            "--bitrate", "64000").stdout.strip()
        results.append({
            "asset": file_spec["asset"],
            "source": source,
            "source_info": probe(source),
            "trimmed_sec": round(len(audio) / SAMPLE_RATE, 3),
            "stats": measure(norm_path),
            "gate": gate_result,
            "encode": encode,
            "opus_bytes": opus_path.stat().st_size,
            "credit": file_spec["credit"],
        })
    return results


def ebur128(path):
    """Integrated loudness and true peak through ffmpeg's scanner."""
    stderr = run(["ffmpeg", "-hide_banner", "-nostats", "-i", path,
                  "-af", "ebur128=peak=true", "-f", "null", "-"]).stderr
    summary = stderr.split("Summary:")[-1]
    integrated = true_peak = None
    for line in summary.splitlines():
        line = line.strip()
        if line.startswith("I:") and integrated is None:
            integrated = float(line.split()[1])
        elif line.startswith("Peak:") and true_peak is None:
            true_peak = float(line.split()[1])
    return integrated, true_peak


def render_pack(pack_dir, out, minutes, seed):
    return oniris_cli("render", "--pack", pack_dir, "--minutes", str(minutes),
                      "--seed", str(seed), "--out", out).stdout.strip()


def verify_mix(spec, pack_dir, work, minutes=3, seed=1, tolerance=0.5):
    """Render the pack and check the mix, not just the individual assets.

    Each bed is normalised to the pack target on its own, so the sum of two beds
    plays back louder than either one. This measures the mix and every bed's
    solo contribution at its manifest gain, which is the number that decides
    whether a support layer is audible or lost.
    """
    target = spec.get("loudnessLufs", -23.0)
    ceiling = spec.get("truePeakDbtp", -1.5)
    mix_path = work / "verify-mix.wav"
    render_pack(pack_dir, mix_path, minutes, seed)
    mix_lufs, mix_tp = ebur128(mix_path)

    manifest = json.loads((pack_dir / "manifest.json").read_text())
    solo = []
    for bed in manifest["beds"]:
        solo_dir = work / f"solo-{bed['id']}"
        if solo_dir.exists():
            shutil.rmtree(solo_dir)
        (solo_dir / "audio").mkdir(parents=True)
        os.symlink(pack_dir / bed["file"], solo_dir / bed["file"])
        single = dict(manifest, beds=[bed], events=[],
                      credits=[c for c in manifest["credits"] if c["asset"] == bed["file"]])
        (solo_dir / "manifest.json").write_text(json.dumps(single))
        path = work / f"solo-{bed['id']}.wav"
        render_pack(solo_dir, path, minutes, seed)
        lufs, true_peak = ebur128(path)
        solo.append({
            "id": bed["id"],
            "gain_db": bed["gainDb"],
            "lufs": lufs,
            "true_peak": true_peak,
            "vs_mix": (round(lufs - mix_lufs, 2)
                       if lufs is not None and mix_lufs is not None else None),
        })

    problems = []
    if mix_lufs is not None and abs(mix_lufs - target) > tolerance:
        problems.append(f"mix {mix_lufs} LUFS is {mix_lufs - target:+.1f} LU from the "
                        f"{target} LUFS pack target (tolerance {tolerance})")
    if mix_tp is not None and mix_tp > ceiling:
        problems.append(f"mix true peak {mix_tp} dBTP exceeds {ceiling} dBTP")
    return {
        "mix": {"lufs": mix_lufs, "true_peak": mix_tp, "target_lufs": target,
                "target_true_peak": ceiling, "minutes": minutes, "seed": seed},
        "beds": solo,
        "problems": problems,
    }


def csv_notes(item):
    """Short audition note for the CSV; the spec's `notes` is often a long
    rationale, so prefer an explicit `csvNotes` and truncate the fallback."""
    if item.get("csvNotes"):
        return item["csvNotes"]
    notes = item.get("notes", "")
    return notes if len(notes) <= 140 else notes[:137].rstrip() + "..."


# ---------------------------------------------------------------- packaging


def pack_license(spec):
    rank = max(LICENSE_RANK.get(c["license"], 9)
               for item in spec.iter_beds_and_events() for c in item)
    for name, value in LICENSE_RANK.items():
        if value == rank:
            return name
    return "CC-BY-4.0"


def credit_lines(asset, credit):
    return [
        f"{asset}",
        f"  Title:    {credit['title']}",
        f"  Author:   {credit['author']}",
        f"  Source:   {credit['url']}",
        f"  License:  {credit['license']} ({CLAIMS.get(credit['license'], 'see source')})",
        f"  Modified: {credit.get('modified', 'trimmed, faded, loudness normalised')}",
        "",
    ]


def write_manifest_and_docs(spec, pack_dir, beds, events):
    license_name = pack_license(spec)
    credits = []
    doc_lines = []

    manifest_beds = []
    for bed in beds:
        manifest_beds.append({
            "id": bed["id"],
            "file": f"audio/{bed['id']}.opus",
            "loop": {"startSample": 0, "endSample": bed["loop_frames"]},
            "gainDb": bed["gain_db"],
            "pan": bed["pan"],
            **({"lfo": bed["lfo"]} if bed["lfo"] else {}),
        })
        credits.append({
            "asset": f"audio/{bed['id']}.opus",
            "author": bed["credit"]["author"],
            "license": bed["credit"]["license"],
            "modified": True,
            "source": bed["credit"]["url"],
            "title": bed["credit"]["title"],
        })
        doc_lines += credit_lines(f"audio/{bed['id']}.opus  ({bed['loop_sec']} s loop)", bed["credit"])

    manifest_events = []
    for event, files in events:
        event_credits = [f["credit"] for f in files]
        # The manifest models event gain as a range [min, max] that the engine
        # samples per instance, not as one gain per file.
        gain = event.get("gainDb", 0.0)
        if isinstance(gain, (int, float)):
            gain = [float(gain), float(gain)]
        elif len(gain) == 1:
            gain = [float(gain[0]), float(gain[0])]
        entry = {
            "id": event["id"],
            "files": [f["asset"] for f in files],
            "gainDb": [float(value) for value in gain],
            "maxVoices": event.get("maxVoices", 1),
            "minIntervalSec": event.get("minIntervalSec", 10.0),
            "maxIntervalSec": event.get("maxIntervalSec", 40.0),
            "pan": event.get("pan", [0.0]),
            "fadeInMs": event.get("fadeInMs", 10),
            "fadeOutMs": event.get("fadeOutMs", 200),
        }
        manifest_events.append(entry)
        for asset_file, credit in zip(files, event_credits):
            credits.append({
                "asset": asset_file["asset"],
                "author": credit["author"],
                "license": credit["license"],
                "modified": True,
                "source": credit["url"],
                "title": credit["title"],
            })
            doc_lines += credit_lines(asset_file["asset"], credit)

    manifest = {
        "schemaVersion": 1,
        "id": spec["id"],
        "name": spec["name"],
        "description": spec["description"],
        "version": spec["version"],
        "license": license_name,
        "tags": spec["tags"],
        "loudnessLufs": spec.get("loudnessLufs", -23.0),
        "truePeakDbtp": spec.get("truePeakDbtp", -1.5),
        "beds": manifest_beds,
        "events": manifest_events,
        "credits": credits,
    }
    (pack_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    license_text = [
        f"{spec['name']['en']} — Oniris sound pack",
        "=" * (len(spec['name']['en']) + 20),
        "",
        f"Pack license: {license_name}",
        f"  {CLAIMS.get(license_name, '')}",
        "",
        "Every audio asset and its attribution:",
        "",
        "\n".join(line for line in doc_lines if line is not None).rstrip(),
        "",
        "Transformation notice: each asset was trimmed, cleaned (DC/rumble removal),",
        "looped with an equal-power crossfade where applicable, and normalised",
        f"to {spec.get('loudnessLufs', -23.0)} LUFS with a true-peak ceiling of",
        f"{spec.get('truePeakDbtp', -1.5)} dBTP, then encoded to Ogg Opus.",
        "",
    ]
    (pack_dir / "LICENSE").write_text("\n".join(license_text))

    readme = [
        f"# {spec['name']['en']}",
        "",
        spec["description"]["en"],
        "",
        f"- Version: {spec['version']}",
        f"- License: {license_name}",
        "- Audio: 48 kHz, Ogg Opus (96 kbps stereo beds, 64 kbps mono events)",
        f"- Loudness: beds normalized to {spec.get('loudnessLufs', -23.0)} LUFS; "
        f"true peak at or below {spec.get('truePeakDbtp', -1.5)} dBTP",
        "",
        "## Loops",
        "",
    ]
    for bed in beds:
        readme.append(
            f"- {bed['id']}: {bed['loop_sec']} s, {bed['crossfade_sec']} s equal-power "
            f"crossfade, seam spectral continuity {bed['seam_spectral']}",
        )
    readme += ["", "## Credits", ""]
    for credit in credits:
        readme.append(
            f"- {Path(credit['asset']).name} — {credit['title']} by {credit['author']} "
            f"({credit['license']}, {credit['source']})",
        )
    readme += ["", "See LICENSE for full attribution and transformation details.", ""]
    (pack_dir / "README.md").write_text("\n".join(readme))
    return manifest


def update_sources_csv(rows):
    """Fill the TODO columns of tools/asset-pipeline/sources.csv from results.

    The CSV logs one row per *source item*, so when several assets are cut from
    the same recording (e.g. two gull bursts) their edits are merged into that
    single row instead of the last one winning.
    """
    path = REPO / "tools" / "asset-pipeline" / "sources.csv"
    with open(path, newline="") as handle:
        reader = csv.DictReader(handle)
        fieldnames = reader.fieldnames
        existing = list(reader)

    merged = {}
    for row in rows:
        key = (row["pack"], row["role"], row["url"])
        if key in merged:
            merged[key]["edits"] = f"{merged[key]['edits']}; {row['edits']}"
        else:
            merged[key] = dict(row)

    updated = 0
    for row in existing:
        key = (row["pack"], row["role"], row["url"])
        if key in merged:
            row.update({k: str(v) for k, v in merged[key].items() if k in fieldnames})
            del merged[key]
            updated += 1
    # A source with no row yet gets appended: the log is meant to be complete,
    # and a new asset silently missing its SHA-256 is the TODO nobody audits.
    for key, row in merged.items():
        existing.append({field: str(row.get(field, "")) for field in fieldnames})
        updated += 1

    with open(path, "w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(existing)
    return updated


# --------------------------------------------------------------------- spec


class Spec(dict):
    def iter_beds_and_events(self):
        for bed in self.get("beds", []):
            yield [bed["credit"]]
        for event in self.get("events", []):
            for file_spec in event["files"]:
                yield [file_spec["credit"]]

    def sources(self):
        out = []
        for bed in self.get("beds", []):
            out.append((bed["id"], bed["source"], bed["credit"]))
        for event in self.get("events", []):
            for file_spec in event["files"]:
                out.append((file_spec["asset"], file_spec["source"], file_spec["credit"]))
        return out


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    build = sub.add_parser("build", help="build a pack from a spec")
    build.add_argument("spec")
    build.add_argument("--out", default=None, help="pack directory (default dist/packs/<id>)")
    build.add_argument("--work", default=None, help="work directory for intermediate WAVs")
    build.add_argument("--only", default=None, help="ingest only this bed/event id")
    build.add_argument("--skip-package", action="store_true", help="stop before oniris pack")
    build.add_argument("--skip-verify", action="store_true", help="skip the mix render check")
    build.add_argument("--no-csv", action="store_true", help="do not update sources.csv")

    verify_cmd = sub.add_parser("verify", help="render the built pack and check the mix")
    verify_cmd.add_argument("spec")
    verify_cmd.add_argument("--out", default=None)
    verify_cmd.add_argument("--work", default=None)
    verify_cmd.add_argument("--minutes", type=int, default=3)
    verify_cmd.add_argument("--seed", type=int, default=1)
    verify_cmd.add_argument("--tolerance", type=float, default=0.5,
                            help="allowed LU deviation from the pack loudness target")

    probe_cmd = sub.add_parser("probe", help="print format and SHA-256 for sources")
    probe_cmd.add_argument("paths", nargs="+")

    args = parser.parse_args()

    if args.command == "probe":
        for path in args.paths:
            info = probe(path)
            print(f"{Path(path).name}: {info['sample_rate']} Hz, {info['channels']}ch, "
                  f"{info['bit_depth']}, {info['duration_sec']} s, sha256 {sha256(path)}")
        return 0

    spec = Spec(json.loads(Path(args.spec).read_text()))
    pack_dir = Path(os.path.expanduser(args.out)) if args.out else REPO / "dist" / "packs" / spec["id"]
    work = Path(os.path.expanduser(args.work)) if args.work else REPO / "dist" / "work" / spec["id"]
    work.mkdir(parents=True, exist_ok=True)

    if args.command == "verify":
        result = verify_mix(spec, pack_dir, work, args.minutes, args.seed, args.tolerance)
        mix = result["mix"]
        print(f"mix  {mix['lufs']} LUFS  TP {mix['true_peak']} dBTP"
              f"  (target {mix['target_lufs']} LUFS / {mix['target_true_peak']} dBTP,"
              f" {mix['minutes']} min seed {mix['seed']})")
        for bed in result["beds"]:
            print(f"  {bed['id']:<8} gain {bed['gain_db']:+6.1f} dB"
                  f"  solo {bed['lufs']} LUFS  ({bed['vs_mix']:+.1f} LU vs mix)")
        for problem in result["problems"]:
            print(f"  FAIL {problem}")
        return 1 if result["problems"] else 0

    # A full build starts from an empty pack directory: a removed bed or event
    # must not survive as an orphan file that packaging would still ship.
    if not args.only and pack_dir.exists() and (REPO / "dist") in pack_dir.parents:
        shutil.rmtree(pack_dir)
    pack_dir.mkdir(parents=True, exist_ok=True)

    target_lufs = spec.get("loudnessLufs", -23.0)
    true_peak = spec.get("truePeakDbtp", -1.5)
    today = datetime.date.today().isoformat()

    beds, events, csv_rows = [], [], []
    for bed in spec.get("beds", []):
        if args.only and bed["id"] != args.only:
            continue
        print(f"== bed {bed['id']}  <- {Path(os.path.expanduser(bed['source'])).name}", flush=True)
        result = ingest_bed(bed, work, pack_dir, target_lufs, true_peak)
        beds.append(result)
        stats = result["stats"]
        print(f"   loop {result['loop_sec']} s  |  {stats['lufs']} LUFS  "
              f"TP {stats['true_peak_dbtp']} dBTP  peak {stats['peak_dbfs']} dBFS", flush=True)
        print(f"   seam {result['seam_delta']} ({result['seam_vs_p99']}x p99.9 step)  "
              f"spectral {result['seam_spectral']}  DC {result['dc_offset_before']} -> cleaned",
              flush=True)
        print(f"   bands {stats['bands_pct']}", flush=True)
        print(f"   opus {result['opus_bytes'] / 1024:.1f} kB  |  {result['gate']}", flush=True)
        info = result["source_info"]
        csv_rows.append({
            "pack": spec["id"],
            "role": "bed",
            "url": bed["credit"]["url"],
            "author": bed["credit"]["author"],
            "license": bed["credit"]["license"],
            "download_date": bed["credit"].get("downloadDate", today),
            "original_file": result["source"].name,
            "sha256": sha256(result["source"]),
            "sample_rate": info["sample_rate"],
            "bit_depth": info["bit_depth"].split()[0],
            "channels": info["channels"],
            "duration_sec": info["duration_sec"],
            "edits": (f"trim {result['trimmed_sec']}s, loop {result['loop_sec']}s "
                      f"xfade {result['crossfade_sec']}s, dc+rumble, -{abs(target_lufs)} LUFS"),
            "notes": csv_notes(bed),
        })

    for event in spec.get("events", []):
        if args.only and event["id"] != args.only:
            continue
        print(f"== event {event['id']}  ({len(event['files'])} file(s))", flush=True)
        files = ingest_event(event, work, pack_dir, target_lufs, true_peak)
        events.append((event, files))
        for asset_file in files:
            stats = asset_file["stats"]
            print(f"   {asset_file['asset']}: {asset_file['trimmed_sec']} s  "
                  f"{stats['lufs']} LUFS  TP {stats['true_peak_dbtp']} dBTP  "
                  f"{asset_file['opus_bytes'] / 1024:.1f} kB  |  {asset_file['gate']}", flush=True)
            info = asset_file["source_info"]
            csv_rows.append({
                "pack": spec["id"],
                "role": "event",
                "url": asset_file["credit"]["url"],
                "author": asset_file["credit"]["author"],
                "license": asset_file["credit"]["license"],
                "download_date": asset_file["credit"].get("downloadDate", today),
                "original_file": asset_file["source"].name,
                "sha256": sha256(asset_file["source"]),
                "sample_rate": info["sample_rate"],
                "bit_depth": info["bit_depth"],
                "channels": info["channels"],
                "duration_sec": info["duration_sec"],
                "edits": f"trim {asset_file['trimmed_sec']}s, fades, -{abs(target_lufs)} LUFS",
                "notes": csv_notes(asset_file["credit"]),
            })

    if not beds and not events:
        raise SystemExit("nothing matched --only")

    manifest = write_manifest_and_docs(spec, pack_dir, beds, events)
    print(f"== manifest {pack_dir / 'manifest.json'}  ({len(manifest['beds'])} beds, "
          f"{len(manifest['events'])} events, license {manifest['license']})", flush=True)

    validate = oniris_cli("validate", pack_dir).stdout
    print(f"== validate OK  {validate.strip()}", flush=True)

    if not args.skip_verify:
        result = verify_mix(spec, pack_dir, work, minutes=3, seed=1)
        mix = result["mix"]
        print(f"== mix  {mix['lufs']} LUFS  TP {mix['true_peak']} dBTP"
              f"  (target {mix['target_lufs']} / {mix['target_true_peak']} dBTP)", flush=True)
        for bed in result["beds"]:
            print(f"   {bed['id']:<8} gain {bed['gain_db']:+6.1f} dB"
                  f"  solo {bed['lufs']} LUFS  ({bed['vs_mix']:+.1f} LU vs mix)", flush=True)
        if result["problems"]:
            for problem in result["problems"]:
                print(f"   FAIL {problem}", flush=True)
            raise SystemExit("mix check failed; tune the spec's gainDb or loudnessLufs")

    if not args.no_csv:
        count = update_sources_csv(csv_rows)
        print(f"== sources.csv updated ({count} row(s))", flush=True)

    if not args.skip_package:
        archive = REPO / "dist" / f"{spec['id']}.onirispack"
        packed = oniris_cli("pack", pack_dir, archive).stdout.strip()
        print(f"== {packed}", flush=True)
        verified = oniris_cli("verify-pack", archive).stdout.strip()
        print(f"== {verified}", flush=True)

    report = work / "report.json"
    report.write_text(json.dumps({
        "spec": spec["id"],
        "built": datetime.datetime.now().isoformat(timespec="seconds"),
        "beds": [{
            "id": bed["id"], "loop_sec": bed["loop_sec"],
            "seam_delta": bed["seam_delta"], "seam_spectral": bed["seam_spectral"],
            "stats": bed["stats"],
        } for bed in beds],
        "events": [{
            "id": event["id"],
            "files": [{"asset": f["asset"], "stats": f["stats"]} for f in files],
        } for event, files in events],
    }, indent=2))
    print(f"== report {report}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
