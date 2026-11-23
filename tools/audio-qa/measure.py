#!/usr/bin/env python3
"""Measure WAV assets against the pack policy.

Reports loudness (EBU R128 via ffmpeg), true peak, sample peak, clipping, DC
offset, stereo correlation, spectral centroid, band energy distribution, and
loop-seam continuity. With --check it exits non-zero when a hard rule is
violated, so it can gate regenerated content.

Requires: python3 + numpy, and ffmpeg on PATH.

Usage:
    tools/audio-qa/measure.py packs/*/audio/*.wav
    tools/audio-qa/measure.py --check packs/*/audio/*.wav
    tools/audio-qa/measure.py --json packs/test-rain/audio/rain.wav
"""

import json
import subprocess
import sys
import wave

try:
    import numpy as np
except ImportError:  # pragma: no cover - environment guidance
    raise SystemExit(
        "numpy is required: pip install -r tools/audio-qa/requirements.txt "
        "(or run with a python that has it, e.g. make qa PYTHON=/usr/bin/python3)",
    )

# Hard limits from docs/PACK_FORMAT.md and the content pipeline plan.
TRUE_PEAK_LIMIT_DBTP = -1.5
MAX_CLIPPED_SAMPLES = 0
MAX_DC_OFFSET = 0.001

BANDS = [
    (0, 20), (20, 60), (60, 120), (120, 300),
    (300, 1000), (1000, 3000), (3000, 8000), (8000, 20000),
]


def load(path):
    with wave.open(path, "rb") as handle:
        channels = handle.getnchannels()
        width = handle.getsampwidth()
        rate = handle.getframerate()
        frames = handle.getnframes()
        raw = handle.readframes(frames)
    if width != 2:
        raise SystemExit(f"{path}: expected 16-bit PCM, got {width * 8}-bit")
    data = np.frombuffer(raw, dtype="<i2").astype(np.float64) / 32768.0
    return data.reshape(-1, channels), rate


def db(value):
    return float(20 * np.log10(max(float(value), 1e-12)))


def lufs(path):
    """Integrated loudness and true peak from ffmpeg's EBU R128 scanner."""
    stderr = subprocess.run(
        ["ffmpeg", "-hide_banner", "-nostats", "-i", path,
         "-af", "ebur128=peak=true", "-f", "null", "-"],
        capture_output=True, text=True, check=False,
    ).stderr
    summary = stderr.split("Summary:")[-1]
    integrated = None
    true_peak = None
    for line in summary.splitlines():
        line = line.strip()
        if line.startswith("I:") and integrated is None:
            integrated = float(line.split()[1])
        elif line.startswith("Peak:") and true_peak is None:
            true_peak = float(line.split()[1])
    return integrated, true_peak


def analyze(path):
    x, rate = load(path)
    mono = x.mean(axis=1)
    peak = float(np.abs(x).max())
    rms = float(np.sqrt((mono ** 2).mean()))

    correlation = None
    side_minus_mid = None
    if x.shape[1] == 2:
        correlation = float(np.corrcoef(x[:, 0], x[:, 1])[0, 1])
        mid = (x[:, 0] + x[:, 1]) / 2
        side = (x[:, 0] - x[:, 1]) / 2
        side_minus_mid = db(np.sqrt((side ** 2).mean())) - db(np.sqrt((mid ** 2).mean()))

    win = 8192 if len(mono) >= 8192 else 1024
    hop = win // 2
    window = np.hanning(win)
    freqs = np.fft.rfftfreq(win, 1 / rate)
    psd = np.zeros(len(freqs))
    centroids = []
    for index in range(0, len(mono) - win, hop):
        mag = np.abs(np.fft.rfft(mono[index:index + win] * window))
        psd += mag ** 2
        if mag.sum() > 0:
            centroids.append(float((freqs * mag).sum() / mag.sum()))
    psd /= psd.sum() if psd.sum() > 0 else 1.0
    bands = {
        f"{low}-{high}": round(100 * float(psd[(freqs >= low) & (freqs < high)].sum()), 1)
        for low, high in BANDS
    }
    flatness = float(np.exp(np.mean(np.log(psd + 1e-20))) / np.mean(psd))

    seam = float(abs(x[0, 0] - x[-1, 0]))
    typical_step = max(float(np.percentile(np.abs(np.diff(x[:, 0])), 99.9)), 1e-9)

    integrated, true_peak = lufs(path)
    return {
        "file": path,
        "seconds": round(len(mono) / rate, 2),
        "channels": int(x.shape[1]),
        "rate": rate,
        "peak_dbfs": round(db(peak), 2),
        "true_peak_dbtp": true_peak,
        "lufs": integrated,
        "rms_dbfs": round(db(rms), 2),
        "crest_db": round(db(peak) - db(rms), 2),
        "clipped_samples": int((np.abs(x) >= 0.999).sum()),
        "dc_offset": [round(float(c.mean()), 6) for c in x.T],
        "stereo_corr": None if correlation is None else round(correlation, 3),
        "side_minus_mid_db": None if side_minus_mid is None else round(side_minus_mid, 2),
        "centroid_hz": round(float(np.mean(centroids))) if centroids else None,
        "flatness": round(flatness, 4),
        "bands_pct": bands,
        "loop_seam_delta": round(seam, 6),
        "loop_seam_vs_p99": round(seam / typical_step, 2),
    }


def violations(result, manifest_lufs=None):
    problems = []
    peak = result["true_peak_dbtp"]
    if peak is not None and peak > TRUE_PEAK_LIMIT_DBTP:
        problems.append(f"true peak {peak} dBTP > {TRUE_PEAK_LIMIT_DBTP} dBTP")
    if result["clipped_samples"] > MAX_CLIPPED_SAMPLES:
        problems.append(f"{result['clipped_samples']} clipped samples")
    for channel, offset in enumerate(result["dc_offset"]):
        if abs(offset) > MAX_DC_OFFSET:
            problems.append(f"DC offset {offset} on channel {channel}")
    if manifest_lufs is not None and result["lufs"] is not None:
        if abs(result["lufs"] - manifest_lufs) > 0.5:
            problems.append(
                f"loudness {result['lufs']} LUFS != manifest {manifest_lufs} LUFS",
            )
    return problems


def main():
    args = sys.argv[1:]
    as_json = "--json" in args
    check = "--check" in args
    paths = [a for a in args if not a.startswith("--")]
    if not paths:
        raise SystemExit(__doc__)

    results = [analyze(path) for path in paths]
    if as_json:
        print(json.dumps(results, indent=2))
    else:
        for result in results:
            print(f"\n== {result['file']}  ({result['seconds']} s, {result['channels']}ch)")
            print(f"   LUFS {result['lufs']}  peak {result['peak_dbfs']} dBFS"
                  f"  true peak {result['true_peak_dbtp']} dBTP"
                  f"  rms {result['rms_dbfs']}  crest {result['crest_db']} dB")
            print(f"   clipped {result['clipped_samples']}  dc {result['dc_offset']}"
                  f"  stereo corr {result['stereo_corr']}"
                  f"  side-mid {result['side_minus_mid_db']} dB")
            print(f"   centroid {result['centroid_hz']} Hz  flatness {result['flatness']}"
                  f"  loop seam {result['loop_seam_delta']}"
                  f" (x{result['loop_seam_vs_p99']} p99.9 step)")
            print("   bands: " + "  ".join(f"{k} {v}%" for k, v in result["bands_pct"].items()))

    if check:
        failures = 0
        for result in results:
            problems = violations(result)
            if problems:
                failures += 1
                print(f"\nFAIL {result['file']}")
                for problem in problems:
                    print(f"  - {problem}")
        if failures:
            print(f"\n{failures} file(s) violate pack policy")
            return 1
        print("\nall files within pack policy")
    return 0


if __name__ == "__main__":
    sys.exit(main())
