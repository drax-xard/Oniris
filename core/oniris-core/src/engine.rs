use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use oniris_pack::{LfoTarget, Manifest};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::audio::{decode_audio, DecodedAudio};
use crate::error::EngineError;
use crate::limiter::Limiter;
use crate::mix::MixSpec;

pub const DEFAULT_MEMORY_LIMIT: usize = 96 * 1024 * 1024;
pub const DEFAULT_BLOCK_FRAMES: usize = 4096;
/// Crossfade applied when the active mix changes during playback.
pub const MIX_CROSSFADE_MS: u32 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Pausing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct RenderStats {
    pub frames_written: usize,
    pub peak: f32,
    pub finished: bool,
}

#[derive(Debug)]
pub struct LoadedPack {
    manifest: Manifest,
    beds: Vec<LoadedBed>,
    events: Vec<LoadedEvent>,
    decoded_bytes: usize,
}

impl LoadedPack {
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn bed_count(&self) -> usize {
        self.beds.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn decoded_bytes(&self) -> usize {
        self.decoded_bytes
    }
}

#[derive(Debug)]
struct LoadedBed {
    audio: DecodedAudio,
    loop_start: usize,
    loop_end: usize,
    gain: f32,
    pan: f32,
    lfo: Option<LfoRuntime>,
}

#[derive(Debug, Clone, Copy)]
struct LfoRuntime {
    target: LfoTarget,
    depth: f32,
    phase_step: f32,
}

#[derive(Debug)]
struct LoadedEvent {
    id: String,
    files: Vec<Arc<DecodedAudio>>,
    min_frames: u64,
    max_frames: u64,
    initial_delay_frames: Option<u64>,
    gain_db: [f32; 2],
    pan: [f32; 2],
    fade_in_frames: u32,
    fade_out_frames: u32,
    max_voices: u8,
}

#[derive(Debug)]
struct ActivePack {
    pack: Arc<LoadedPack>,
    gain: f32,
    bed_positions: Vec<usize>,
    bed_lfo_phases: Vec<f32>,
    event_delays: Vec<u64>,
    fade: GainFade,
    /// Old layer that is fading out after a mix change.
    fading_out: bool,
}

#[derive(Debug)]
struct EventVoice {
    pack_id: String,
    event_id: String,
    audio: Arc<DecodedAudio>,
    position: usize,
    gain: f32,
    pan_l: f32,
    pan_r: f32,
    fade_in: usize,
    fade_out: usize,
    total: usize,
    /// Fades the voice when its pack leaves the mix.
    fade: GainFade,
}

/// Linear gain ramp used for mix crossfades. `remaining == 0` means steady.
#[derive(Debug, Clone, Copy)]
struct GainFade {
    gain: f32,
    step: f32,
    remaining: u32,
    target: f32,
}

impl GainFade {
    fn steady(gain: f32) -> Self {
        Self {
            gain,
            step: 0.0,
            remaining: 0,
            target: gain,
        }
    }

    fn ramp(from: f32, to: f32, frames: u32) -> Self {
        if frames == 0 {
            Self::steady(to)
        } else {
            Self {
                gain: from,
                step: (to - from) / frames as f32,
                remaining: frames,
                target: to,
            }
        }
    }

    fn gain_at(&self, frame: usize) -> f32 {
        if self.remaining == 0 {
            self.gain
        } else {
            (self.gain + self.step * frame as f32).clamp(0.0, 1.0)
        }
    }

    fn advance(&mut self, frames: usize) {
        if self.remaining == 0 {
            return;
        }
        let frames = frames.min(self.remaining as usize) as u32;
        self.gain += self.step * frames as f32;
        self.remaining -= frames;
        if self.remaining == 0 {
            self.gain = self.target;
            self.step = 0.0;
        }
    }

    fn is_finished(&self) -> bool {
        self.remaining == 0
    }

    fn is_silent(&self) -> bool {
        self.remaining == 0 && self.gain <= 1e-4
    }
}

pub struct Engine {
    sample_rate: u32,
    packs: HashMap<String, Arc<LoadedPack>>,
    active: Vec<ActivePack>,
    voices: Vec<EventVoice>,
    limiter: Limiter,
    master_gain: f32,
    state: TransportState,
    transport_gain: f32,
    transport_target: f32,
    transport_step: f32,
    transport_remaining: u32,
    fade_in_frames: u32,
    pause_fade_frames: u32,
    timer_active: bool,
    timer_remaining: u64,
    timer_fade_frames: u64,
    finished: bool,
    rng: ChaCha8Rng,
    memory_limit: usize,
    scratch: Vec<f32>,
}

impl Engine {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_seed(sample_rate, 0x0D1_215)
    }

    pub fn with_seed(sample_rate: u32, seed: u64) -> Self {
        let sample_rate = sample_rate.max(8_000);
        Self {
            sample_rate,
            packs: HashMap::new(),
            active: Vec::new(),
            voices: Vec::new(),
            limiter: Limiter::new(sample_rate),
            master_gain: 1.0,
            state: TransportState::Stopped,
            transport_gain: 0.0,
            transport_target: 0.0,
            transport_step: 0.0,
            transport_remaining: 0,
            fade_in_frames: sample_rate * 10,
            pause_fade_frames: sample_rate / 5,
            timer_active: false,
            timer_remaining: 0,
            timer_fade_frames: 0,
            finished: false,
            rng: ChaCha8Rng::seed_from_u64(seed),
            memory_limit: DEFAULT_MEMORY_LIMIT,
            scratch: Vec::new(),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn state(&self) -> TransportState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        matches!(
            self.state,
            TransportState::Playing | TransportState::Pausing
        )
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn master_gain(&self) -> f32 {
        self.master_gain
    }

    pub fn active_pack_count(&self) -> usize {
        self.active.iter().filter(|pack| !pack.fading_out).count()
    }

    pub fn decoded_bytes(&self) -> usize {
        self.packs.values().map(|pack| pack.decoded_bytes).sum()
    }

    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }

    pub fn set_memory_limit(&mut self, bytes: usize) {
        self.memory_limit = bytes;
    }

    pub fn set_fade_in(&mut self, duration: Duration) {
        self.fade_in_frames = duration.as_secs_f32().mul_add(self.sample_rate as f32, 0.0) as u32;
    }

    pub fn set_pause_fade(&mut self, duration: Duration) {
        self.pause_fade_frames =
            duration.as_secs_f32().mul_add(self.sample_rate as f32, 0.0) as u32;
    }

    pub fn loaded_pack(&self, id: &str) -> Option<&LoadedPack> {
        self.packs.get(id).map(Arc::as_ref)
    }

    pub fn loaded_pack_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.packs.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Decode and load a pack directory. The pack is not audible until `set_mix`.
    pub fn load_pack(&mut self, dir: &Path) -> Result<String, EngineError> {
        let manifest = Manifest::load(dir)?;
        manifest.validate_files(dir)?;
        let id = manifest.id.clone();

        if self.packs.contains_key(&id) {
            return Err(EngineError::InvalidMix(format!(
                "pack {id:?} is already loaded"
            )));
        }

        let mut decoded_bytes = 0usize;
        let mut beds = Vec::with_capacity(manifest.beds.len());
        for bed in &manifest.beds {
            let audio = decode_audio(&dir.join(&bed.file), self.sample_rate)?;
            let scale = self.sample_rate as f64 / audio.source_sample_rate as f64;
            let loop_start = (bed.loop_region.start_sample as f64 * scale).round() as usize;
            let loop_end = (bed.loop_region.end_sample as f64 * scale).round() as usize;

            if loop_end > audio.frames {
                return Err(EngineError::InvalidAudio(format!(
                    "bed {:?}: loop end {} exceeds {} decoded frames",
                    bed.id, loop_end, audio.frames
                )));
            }
            if loop_end <= loop_start {
                return Err(EngineError::InvalidAudio(format!(
                    "bed {:?}: invalid loop range {}..{}",
                    bed.id, loop_start, loop_end
                )));
            }

            let audio = audio.into_stereo();
            decoded_bytes = decoded_bytes.saturating_add(audio.bytes());
            beds.push(LoadedBed {
                audio,
                loop_start,
                loop_end,
                gain: db_to_linear(bed.gain_db),
                pan: bed.pan.clamp(-1.0, 1.0),
                lfo: bed.lfo.map(|lfo| LfoRuntime {
                    target: lfo.target,
                    depth: lfo.depth,
                    phase_step: std::f32::consts::TAU
                        / (lfo.period_sec.max(1.0) * self.sample_rate as f32),
                }),
            });
        }

        let mut events = Vec::with_capacity(manifest.events.len());
        for event in &manifest.events {
            let mut files = Vec::with_capacity(event.files.len());
            for file in &event.files {
                let audio = decode_audio(&dir.join(file), self.sample_rate)?;
                decoded_bytes = decoded_bytes.saturating_add(audio.bytes());
                files.push(Arc::new(audio));
            }

            let min_frames = seconds_to_frames(event.min_interval_sec, self.sample_rate);
            let max_frames =
                seconds_to_frames(event.max_interval_sec, self.sample_rate).max(min_frames);
            events.push(LoadedEvent {
                id: event.id.clone(),
                files,
                min_frames,
                max_frames,
                initial_delay_frames: event
                    .initial_delay_sec
                    .map(|seconds| seconds_to_frames(seconds, self.sample_rate)),
                gain_db: event.gain_db,
                pan: event.pan,
                fade_in_frames: ms_to_frames(event.fade_in_ms, self.sample_rate),
                fade_out_frames: ms_to_frames(event.fade_out_ms, self.sample_rate),
                max_voices: event.max_voices,
            });
        }

        let used = self.decoded_bytes().saturating_add(decoded_bytes);
        if used > self.memory_limit {
            return Err(EngineError::MemoryBudget {
                used,
                limit: self.memory_limit,
            });
        }

        self.packs.insert(
            id.clone(),
            Arc::new(LoadedPack {
                manifest,
                beds,
                events,
                decoded_bytes,
            }),
        );
        Ok(id)
    }

    pub fn unload_pack(&mut self, id: &str) -> bool {
        let removed = self.packs.remove(id).is_some();
        if removed {
            self.active.retain(|pack| pack.pack.manifest.id != id);
            self.voices.retain(|voice| voice.pack_id != id);
        }
        removed
    }

    /// Replace the active mix. All pack ids must already be loaded.
    ///
    /// While playing, the old layers fade out and the new layers fade in over
    /// [`MIX_CROSSFADE_MS`]; loop positions are preserved for unchanged packs
    /// only if `set_mix` is not called again. No allocation or decoding happens
    /// here beyond building the new layer state.
    pub fn set_mix(&mut self, mix: &MixSpec) -> Result<(), EngineError> {
        let crossfade = self.is_playing();
        let fade_frames = ms_to_frames(MIX_CROSSFADE_MS, self.sample_rate).max(1);
        let mut active = Vec::new();
        let mut requested = HashSet::new();
        let mut seen = HashSet::new();

        for entry in &mix.packs {
            if !seen.insert(entry.id.clone()) {
                return Err(EngineError::InvalidMix(format!(
                    "duplicate pack id {:?}",
                    entry.id
                )));
            }
            let pack = self
                .packs
                .get(&entry.id)
                .cloned()
                .ok_or_else(|| EngineError::PackNotLoaded(entry.id.clone()))?;
            if entry.muted {
                continue;
            }
            requested.insert(entry.id.clone());

            let mut bed_positions = Vec::with_capacity(pack.beds.len());
            let mut bed_lfo_phases = Vec::with_capacity(pack.beds.len());
            for bed in &pack.beds {
                bed_positions.push(self.rng.gen_range(bed.loop_start..bed.loop_end));
                bed_lfo_phases.push(self.rng.gen_range(0.0..std::f32::consts::TAU));
            }

            let mut event_delays = Vec::with_capacity(pack.events.len());
            for event in &pack.events {
                let delay = event
                    .initial_delay_frames
                    .unwrap_or_else(|| self.rng.gen_range(event.min_frames..=event.max_frames));
                event_delays.push(delay);
            }

            active.push(ActivePack {
                pack,
                gain: db_to_linear(entry.gain_db),
                bed_positions,
                bed_lfo_phases,
                event_delays,
                fade: if crossfade {
                    GainFade::ramp(0.0, 1.0, fade_frames)
                } else {
                    GainFade::steady(1.0)
                },
                fading_out: false,
            });
        }

        if crossfade {
            // Keep the previous layers running for the duration of the fade.
            for layer in &mut self.active {
                if layer.fading_out {
                    continue;
                }
                layer.fading_out = true;
                layer.fade = GainFade::ramp(layer.fade.gain_at(0), 0.0, fade_frames);
            }
            // Voices belonging to packs that left the mix fade with their layer
            // instead of being cut abruptly.
            for voice in &mut self.voices {
                if !requested.contains(&voice.pack_id) {
                    voice.fade = GainFade::ramp(voice.fade.gain_at(0), 0.0, fade_frames);
                }
            }
            self.active.extend(active);
        } else {
            self.active = active;
            self.voices.clear();
        }

        self.master_gain = mix.master_gain.clamp(0.0, 1.5);
        Ok(())
    }

    pub fn set_pack_gain(&mut self, id: &str, gain: f32) -> Result<(), EngineError> {
        let mut found = false;
        for pack in &mut self.active {
            if !pack.fading_out && pack.pack.manifest.id == id {
                pack.gain = gain.clamp(0.0, 2.0);
                found = true;
            }
        }
        if found {
            Ok(())
        } else {
            Err(EngineError::PackNotLoaded(id.to_string()))
        }
    }

    pub fn set_pack_gain_db(&mut self, id: &str, gain_db: f32) -> Result<(), EngineError> {
        self.set_pack_gain(id, db_to_linear(gain_db))
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain.clamp(0.0, 1.5);
    }

    pub fn play(&mut self) {
        if self.state == TransportState::Playing {
            return;
        }
        if self.finished {
            self.finished = false;
            self.timer_active = false;
            self.timer_remaining = 0;
        }
        self.state = TransportState::Playing;
        self.transport_target = 1.0;
        self.transport_remaining = self.fade_in_frames.max(1);
        self.transport_step =
            (self.transport_target - self.transport_gain) / self.transport_remaining as f32;
        self.limiter.reset();
    }

    pub fn pause(&mut self) {
        if !matches!(
            self.state,
            TransportState::Playing | TransportState::Pausing
        ) {
            return;
        }
        if self.transport_gain <= 0.0001 {
            self.transport_gain = 0.0;
            self.state = TransportState::Paused;
            return;
        }
        self.state = TransportState::Pausing;
        self.transport_target = 0.0;
        self.transport_remaining = self.pause_fade_frames.max(1);
        self.transport_step =
            (self.transport_target - self.transport_gain) / self.transport_remaining as f32;
    }

    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.transport_gain = 0.0;
        self.transport_target = 0.0;
        self.transport_remaining = 0;
        self.finished = false;
        self.timer_active = false;
        self.timer_remaining = 0;
        self.voices.clear();
        self.limiter.reset();
    }

    pub fn set_sleep_timer(&mut self, duration: Duration, fade: Duration) {
        let total = seconds_to_frames_f64(duration.as_secs_f64(), self.sample_rate);
        if total == 0 {
            self.clear_sleep_timer();
            return;
        }
        let fade_frames = seconds_to_frames_f64(fade.as_secs_f64(), self.sample_rate);
        self.timer_remaining = total;
        self.timer_fade_frames = fade_frames.min(total);
        self.timer_active = true;
        self.finished = false;
    }

    pub fn clear_sleep_timer(&mut self) {
        self.timer_active = false;
        self.timer_remaining = 0;
        self.timer_fade_frames = 0;
    }

    pub fn sleep_timer_remaining(&self) -> Option<Duration> {
        if !self.timer_active {
            return None;
        }
        Some(Duration::from_secs_f64(
            self.timer_remaining as f64 / self.sample_rate as f64,
        ))
    }

    /// Render interleaved stereo i16 samples. The output is always zeroed first.
    pub fn render_i16(&mut self, out: &mut [i16], frames: usize) -> RenderStats {
        let frames = frames.min(out.len() / 2);
        if frames == 0 {
            return RenderStats {
                frames_written: 0,
                peak: 0.0,
                finished: self.finished,
            };
        }
        out[..frames * 2].fill(0);

        let mut stats = RenderStats {
            frames_written: frames,
            peak: 0.0,
            finished: self.finished,
        };

        if !matches!(
            self.state,
            TransportState::Playing | TransportState::Pausing
        ) {
            return stats;
        }

        self.scratch.clear();
        self.scratch.resize(frames * 2, 0.0);

        let scratch = &mut self.scratch;
        let voices = &mut self.voices;
        let rng = &mut self.rng;

        for active in &mut self.active {
            let pack = &active.pack;
            let layer_fade = active.fade;

            // Event scheduling happens at block boundaries. A block is ~85 ms
            // at 48 kHz, which is far below the perceptual threshold for
            // thunder/fire crackles. Fading layers do not spawn new events.
            if !active.fading_out && layer_fade.is_finished() {
                for (index, event) in pack.events.iter().enumerate() {
                    let delay = &mut active.event_delays[index];
                    if *delay <= frames as u64 {
                        let sounding = voices
                            .iter()
                            .filter(|voice| {
                                voice.pack_id == pack.manifest.id && voice.event_id == event.id
                            })
                            .count();

                        if sounding < event.max_voices as usize {
                            let file = &event.files[rng.gen_range(0..event.files.len())];
                            let gain_db = rng.gen_range(event.gain_db[0]..=event.gain_db[1]);
                            let pan = rng.gen_range(event.pan[0]..=event.pan[1]);
                            let (pan_l, pan_r) = pan_gains(pan);
                            voices.push(EventVoice {
                                pack_id: pack.manifest.id.clone(),
                                event_id: event.id.clone(),
                                audio: file.clone(),
                                position: 0,
                                gain: db_to_linear(gain_db),
                                pan_l,
                                pan_r,
                                fade_in: event.fade_in_frames as usize,
                                fade_out: event.fade_out_frames as usize,
                                total: file.frames,
                                fade: GainFade::steady(1.0),
                            });
                        }

                        *delay = rng.gen_range(event.min_frames..=event.max_frames);
                    } else {
                        *delay -= frames as u64;
                    }
                }
            }

            for (index, bed) in pack.beds.iter().enumerate() {
                let position = &mut active.bed_positions[index];
                let phase = &mut active.bed_lfo_phases[index];
                let base_gain = active.gain * bed.gain;
                let (base_l, base_r) = pan_gains(bed.pan);

                match bed.lfo {
                    None => {
                        let left_gain = base_gain * base_l * I16_SCALE;
                        let right_gain = base_gain * base_r * I16_SCALE;
                        for frame in 0..frames {
                            let layer_gain = layer_fade.gain_at(frame);
                            let sample = *position * 2;
                            scratch[frame * 2] +=
                                bed.audio.samples[sample] as f32 * left_gain * layer_gain;
                            scratch[frame * 2 + 1] +=
                                bed.audio.samples[sample + 1] as f32 * right_gain * layer_gain;

                            *position += 1;
                            if *position >= bed.loop_end {
                                *position = bed.loop_start;
                            }
                        }
                    }
                    Some(lfo) => {
                        for frame in 0..frames {
                            let layer_gain = layer_fade.gain_at(frame);
                            let modulation = (*phase).sin() * lfo.depth;
                            let (left_gain, right_gain) = match lfo.target {
                                LfoTarget::Gain => {
                                    let gain = db_to_linear(modulation);
                                    (
                                        base_gain * base_l * gain * I16_SCALE,
                                        base_gain * base_r * gain * I16_SCALE,
                                    )
                                }
                                LfoTarget::Pan => {
                                    let (l, r) = pan_gains((bed.pan + modulation).clamp(-1.0, 1.0));
                                    (base_gain * l * I16_SCALE, base_gain * r * I16_SCALE)
                                }
                            };

                            let sample = *position * 2;
                            scratch[frame * 2] +=
                                bed.audio.samples[sample] as f32 * left_gain * layer_gain;
                            scratch[frame * 2 + 1] +=
                                bed.audio.samples[sample + 1] as f32 * right_gain * layer_gain;

                            *position += 1;
                            if *position >= bed.loop_end {
                                *position = bed.loop_start;
                            }

                            *phase += lfo.phase_step;
                            if *phase >= std::f32::consts::TAU {
                                *phase -= std::f32::consts::TAU;
                            }
                        }
                    }
                }
            }

            active.fade.advance(frames);
        }

        // Drop layers whose fade-out has completed.
        self.active
            .retain(|active| !(active.fading_out && active.fade.is_silent()));

        let mut voice_index = 0;
        while voice_index < voices.len() {
            let voice = &mut voices[voice_index];
            if voice.position >= voice.total || voice.fade.is_silent() {
                voices.swap_remove(voice_index);
                continue;
            }

            let count = (voice.total - voice.position).min(frames);
            let channels = voice.audio.channels as usize;
            let voice_fade = voice.fade;
            for frame in 0..count {
                let position = voice.position + frame;
                let envelope = fade_envelope(position, voice.total, voice.fade_in, voice.fade_out);
                let gain = voice.gain * envelope * I16_SCALE * voice_fade.gain_at(frame);
                let base = position * channels;

                if channels == 1 {
                    let sample = voice.audio.samples[base] as f32 * gain;
                    scratch[frame * 2] += sample * voice.pan_l;
                    scratch[frame * 2 + 1] += sample * voice.pan_r;
                } else {
                    scratch[frame * 2] += voice.audio.samples[base] as f32 * gain * voice.pan_l;
                    scratch[frame * 2 + 1] +=
                        voice.audio.samples[base + 1] as f32 * gain * voice.pan_r;
                }
            }

            voice.position += count;
            voice.fade.advance(count);
            if voice.position >= voice.total || voice.fade.is_silent() {
                voices.swap_remove(voice_index);
            } else {
                voice_index += 1;
            }
        }

        let limiter = &mut self.limiter;
        let mut transport_gain = self.transport_gain;
        let mut transport_remaining = self.transport_remaining;
        let transport_step = self.transport_step;
        let transport_target = self.transport_target;
        let mut state = self.state;
        let mut timer_active = self.timer_active;
        let mut timer_remaining = self.timer_remaining;
        let timer_fade_frames = self.timer_fade_frames;
        let master_gain = self.master_gain;
        let mut finished = self.finished;
        let mut peak = 0.0f32;

        for frame in 0..frames {
            if transport_remaining > 0 {
                transport_gain += transport_step;
                transport_remaining -= 1;
                if transport_remaining == 0 {
                    transport_gain = transport_target;
                }
            }

            let timer_gain = if timer_active {
                if timer_remaining == 0 {
                    timer_active = false;
                    state = TransportState::Paused;
                    transport_gain = 0.0;
                    transport_remaining = 0;
                    finished = true;
                    0.0
                } else {
                    let gain = if timer_fade_frames > 0 && timer_remaining <= timer_fade_frames {
                        timer_remaining as f32 / timer_fade_frames as f32
                    } else {
                        1.0
                    };
                    timer_remaining -= 1;
                    if timer_remaining == 0 {
                        timer_active = false;
                        state = TransportState::Paused;
                        transport_gain = 0.0;
                        transport_remaining = 0;
                        finished = true;
                    }
                    gain
                }
            } else {
                1.0
            };

            let gain = transport_gain * timer_gain * master_gain;
            let (left, right) =
                limiter.process(scratch[frame * 2] * gain, scratch[frame * 2 + 1] * gain);
            let left = to_i16(left);
            let right = to_i16(right);
            out[frame * 2] = left;
            out[frame * 2 + 1] = right;
            peak = peak
                .max((left as f32 / i16::MAX as f32).abs())
                .max((right as f32 / i16::MAX as f32).abs());
        }

        if state == TransportState::Pausing && transport_remaining == 0 && transport_gain <= 0.0001
        {
            state = TransportState::Paused;
        }

        self.transport_gain = transport_gain;
        self.transport_remaining = transport_remaining;
        self.state = state;
        self.timer_active = timer_active;
        self.timer_remaining = timer_remaining;
        self.finished = finished;

        stats.peak = peak;
        stats.finished = finished;
        stats
    }

    pub fn block_frames(&self) -> usize {
        DEFAULT_BLOCK_FRAMES
    }

    /// Samples of latency the master limiter adds to the rendered signal.
    ///
    /// Shells can use this when sizing output buffers or interpreting underrun
    /// counts; it is a fixed 5 ms at any sample rate.
    pub fn output_latency_frames(&self) -> usize {
        self.limiter.latency_frames()
    }

    pub fn sample_rate_f32(&self) -> f32 {
        self.sample_rate as f32
    }
}

fn seconds_to_frames(seconds: f32, sample_rate: u32) -> u64 {
    seconds_to_frames_f64(seconds as f64, sample_rate)
}

fn seconds_to_frames_f64(seconds: f64, sample_rate: u32) -> u64 {
    if seconds <= 0.0 {
        0
    } else {
        (seconds * sample_rate as f64).round() as u64
    }
}

fn ms_to_frames(milliseconds: u32, sample_rate: u32) -> u32 {
    ((milliseconds as u64 * sample_rate as u64) / 1000) as u32
}

const I16_SCALE: f32 = 1.0 / 32768.0;

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Simple stereo balance law: center is unity on both channels, edges mute the
/// opposite channel. This avoids the 3 dB center dip of naive constant-power
/// panning on stereo material.
fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    if pan < 0.0 {
        (1.0, 1.0 + pan)
    } else {
        (1.0 - pan, 1.0)
    }
}

fn fade_envelope(position: usize, total: usize, fade_in: usize, fade_out: usize) -> f32 {
    let fade_in_gain = if fade_in > 0 {
        (position as f32 / fade_in as f32).min(1.0)
    } else {
        1.0
    };
    let remaining = total.saturating_sub(position);
    let fade_out_gain = if fade_out > 0 {
        (remaining as f32 / fade_out as f32).min(1.0)
    } else {
        1.0
    };
    fade_in_gain.min(fade_out_gain)
}

fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mix::MixEntry;
    use hound::{WavSpec, WavWriter};
    use std::fs;

    fn write_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[i16]) {
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = WavWriter::create(path, spec).unwrap();
        for sample in samples {
            writer.write_sample(*sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    fn make_pack(dir: &Path, bed_frames: usize, event_frames: usize) {
        make_pack_with_id(dir, "test-pack", bed_frames, event_frames);
    }

    fn make_pack_with_id(dir: &Path, id: &str, bed_frames: usize, event_frames: usize) {
        fs::create_dir_all(dir.join("audio")).unwrap();
        let bed: Vec<i16> = (0..bed_frames)
            .flat_map(|_| [8_000i16, -8_000i16])
            .collect();
        write_wav(&dir.join("audio/bed.wav"), 48_000, 2, &bed);

        let event: Vec<i16> = (0..event_frames).map(|_| 12_000i16).collect();
        write_wav(&dir.join("audio/event.wav"), 48_000, 1, &event);

        let manifest = serde_json::json!({
            "schemaVersion": 1,
            "id": id,
            "name": { "en": "Test Pack" },
            "version": "0.1.0",
            "license": "CC0-1.0",
            "beds": [{
                "id": "bed",
                "file": "audio/bed.wav",
                "loop": { "startSample": 0, "endSample": bed_frames },
                "gainDb": 0.0
            }],
            "events": [{
                "id": "event",
                "files": ["audio/event.wav"],
                "minIntervalSec": 1.0,
                "maxIntervalSec": 1.0,
                "initialDelaySec": 0.0,
                "gainDb": [-6.0, -6.0],
                "fadeInMs": 0,
                "fadeOutMs": 0,
                "maxVoices": 1
            }]
        });
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn make_dc_pack(dir: &Path, id: &str, level: i16, frames: usize) {
        fs::create_dir_all(dir.join("audio")).unwrap();
        let bed: Vec<i16> = std::iter::repeat_n(level, frames)
            .flat_map(|sample| [sample, sample])
            .collect();
        write_wav(&dir.join("audio/bed.wav"), 48_000, 2, &bed);
        let manifest = serde_json::json!({
            "schemaVersion": 1,
            "id": id,
            "name": { "en": "DC Pack" },
            "version": "0.1.0",
            "license": "CC0-1.0",
            "beds": [{
                "id": "bed",
                "file": "audio/bed.wav",
                "loop": { "startSample": 0, "endSample": frames },
                "gainDb": 0.0
            }]
        });
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn mix(entries: &[(&str, f32)]) -> MixSpec {
        MixSpec::new(
            entries
                .iter()
                .map(|(id, gain_db)| MixEntry {
                    id: (*id).to_string(),
                    gain_db: *gain_db,
                    muted: false,
                })
                .collect(),
        )
    }

    fn load_and_play(engine: &mut Engine, mix_spec: &MixSpec, dir: &Path) -> String {
        let id = engine.load_pack(dir).unwrap();
        engine.set_mix(mix_spec).unwrap();
        engine.play();
        id
    }

    fn peak_abs(samples: &[i16]) -> f32 {
        samples.iter().fold(0.0f32, |peak, sample| {
            peak.max(sample.abs() as f32 / 32767.0)
        })
    }

    #[test]
    fn loads_and_renders_a_pack() {
        let dir = tempfile::tempdir().unwrap();
        make_pack(dir.path(), 48_000, 4_800);

        let mut engine = Engine::new(48_000);
        engine.set_fade_in(Duration::ZERO);
        let id = engine.load_pack(dir.path()).unwrap();
        assert_eq!(id, "test-pack");
        assert!(engine.decoded_bytes() > 0);

        engine.set_mix(&mix(&[("test-pack", 0.0)])).unwrap();
        engine.play();

        let mut out = vec![0i16; 48_000 * 2];
        let stats = engine.render_i16(&mut out, 48_000);
        assert_eq!(stats.frames_written, 48_000);
        assert!(stats.peak > 0.1);
        assert!(!stats.finished);
    }

    #[test]
    fn loads_and_renders_an_opus_pack() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("audio")).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tone-stereo.opus"),
            dir.path().join("audio/bed.opus"),
        )
        .unwrap();
        let manifest = serde_json::json!({
            "schemaVersion": 1,
            "id": "opus-pack",
            "name": { "en": "Opus Pack" },
            "version": "0.1.0",
            "license": "CC0-1.0",
            "beds": [{
                "id": "bed",
                "file": "audio/bed.opus",
                "loop": { "startSample": 0, "endSample": 48_000 },
                "gainDb": 0.0
            }]
        });
        fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let mut engine = Engine::new(48_000);
        engine.set_fade_in(Duration::ZERO);
        let id = load_and_play(&mut engine, &mix(&[("opus-pack", 0.0)]), dir.path());
        let mut out = vec![0i16; 24_000 * 2];
        let stats = engine.render_i16(&mut out, 24_000);
        assert_eq!(id, "opus-pack");
        assert!(stats.peak > 0.02, "peak={}", stats.peak);
    }

    #[test]
    fn mix_change_crossfades_instead_of_stepping() {
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        make_dc_pack(dir_a.path(), "dc-a", 8_000, 48_000);
        make_dc_pack(dir_b.path(), "dc-b", -8_000, 48_000);

        let mut engine = Engine::new(48_000);
        engine.set_fade_in(Duration::ZERO);
        engine.load_pack(dir_a.path()).unwrap();
        engine.load_pack(dir_b.path()).unwrap();
        engine.set_mix(&mix(&[("dc-a", 0.0)])).unwrap();
        engine.play();

        let mut out = vec![0i16; 4_096 * 2];
        engine.render_i16(&mut out, 4_096);
        assert!(out[out.len() - 2] > 7_000);

        engine.set_mix(&mix(&[("dc-b", 0.0)])).unwrap();
        assert_eq!(engine.active_pack_count(), 1);
        let mut transition = vec![0i16; 8_192 * 2];
        engine.render_i16(&mut transition, 8_192);

        let crossfade = ms_to_frames(MIX_CROSSFADE_MS, 48_000) as usize;
        let mut joined = out[out.len() - 2..].to_vec();
        joined.extend_from_slice(&transition[..crossfade * 2]);
        let max_step = joined
            .chunks_exact(2)
            .zip(joined[2..].chunks_exact(2))
            .fold(0i32, |max, (a, b)| {
                max.max((b[0] as i32 - a[0] as i32).abs())
            });
        assert!(max_step < 500, "max step={max_step}");

        assert!(transition[(crossfade + 1) * 2] < -7_000);
        assert!(
            transition[..crossfade * 2]
                .chunks_exact(2)
                .any(|frame| frame[0].abs() < 7_000),
            "expected intermediate levels during the crossfade"
        );
        // Once the old layer has faded out it is removed.
        let mut rest = vec![0i16; 4_096 * 2];
        engine.render_i16(&mut rest, 4_096);
        assert_eq!(engine.active_pack_count(), 1);
        assert!(rest[rest.len() - 2] < -7_000);
    }

    #[test]
    fn limiter_caps_a_hot_multi_pack_mix() {
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        make_dc_pack(dir_a.path(), "hot-a", i16::MAX, 48_000);
        make_dc_pack(dir_b.path(), "hot-b", i16::MAX, 48_000);

        let mut engine = Engine::new(48_000);
        engine.set_fade_in(Duration::ZERO);
        engine.load_pack(dir_a.path()).unwrap();
        engine.load_pack(dir_b.path()).unwrap();
        engine
            .set_mix(&mix(&[("hot-a", 0.0), ("hot-b", 0.0)]))
            .unwrap();
        engine.play();

        let mut out = vec![0i16; 48_000 * 2];
        let stats = engine.render_i16(&mut out, 48_000);
        // -1.5 dBFS ceiling with a small allowance for the dB conversion.
        assert!(stats.peak <= 0.842, "peak={}", stats.peak);
        assert!(stats.peak > 0.8, "peak={}", stats.peak);
        assert!(peak_abs(&out) <= 0.842);
        assert!(out.iter().all(|sample| *sample != i16::MAX));
    }

    #[test]
    fn stopped_engine_renders_silence() {
        let mut engine = Engine::new(48_000);
        let mut out = vec![123i16; 32];
        let stats = engine.render_i16(&mut out, 16);
        assert_eq!(stats.frames_written, 16);
        assert!(out.iter().all(|sample| *sample == 0));
    }

    #[test]
    fn timer_fades_and_finishes() {
        let dir = tempfile::tempdir().unwrap();
        make_pack(dir.path(), 48_000, 4_800);

        let mut engine = Engine::new(48_000);
        engine.set_fade_in(Duration::ZERO);
        let id = engine.load_pack(dir.path()).unwrap();
        engine
            .set_mix(&MixSpec::new(vec![MixEntry {
                id,
                gain_db: 0.0,
                muted: false,
            }]))
            .unwrap();
        engine.set_sleep_timer(Duration::from_secs(1), Duration::from_secs(1));
        engine.play();

        let mut out = vec![0i16; 52_800 * 2];
        let stats = engine.render_i16(&mut out, 52_800);
        assert!(stats.finished);
        assert!(stats.peak < 0.5, "peak={}", stats.peak);
        assert_eq!(engine.state(), TransportState::Paused);
        // The limiter's 5 ms lookahead holds back the final, nearly silent
        // samples of the fade; after that the tail is exactly zero.
        let tail = &out[out.len() - 4_800 * 2..];
        assert!(tail.iter().all(|sample| sample.abs() <= 64), "{tail:?}");
        assert!(tail[240 * 2..].iter().all(|sample| *sample == 0));
    }

    #[test]
    fn loop_wraps_and_keeps_playing() {
        let dir = tempfile::tempdir().unwrap();
        make_pack(dir.path(), 48_000, 4_800);

        let mut engine = Engine::new(48_000);
        engine.set_fade_in(Duration::ZERO);
        let id = engine.load_pack(dir.path()).unwrap();
        engine
            .set_mix(&MixSpec::new(vec![MixEntry {
                id,
                gain_db: 0.0,
                muted: false,
            }]))
            .unwrap();
        engine.play();

        let mut out = vec![0i16; 96_000 * 2];
        engine.render_i16(&mut out, 96_000);
        assert!(out[24_000 * 2] != 0);
        assert!(out[72_000 * 2] != 0);
    }

    #[test]
    fn rejects_unknown_pack_in_mix() {
        let mut engine = Engine::new(48_000);
        let result = engine.set_mix(&MixSpec::new(vec![MixEntry {
            id: "missing".into(),
            gain_db: 0.0,
            muted: false,
        }]));
        assert!(matches!(result, Err(EngineError::PackNotLoaded(_))));
    }
}
