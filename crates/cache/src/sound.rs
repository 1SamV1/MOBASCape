//! Synthesised sound effects (cache index 4).
//!
//! OSRS sound effects are not recordings: each is up to ten instruments,
//! and each instrument is a set of oscillators shaped by pitch and volume
//! envelopes, optional vibrato/tremolo, gating, an echo and an IIR filter.
//! This is a port of the reference client's `SoundEffect`, `Instrument`,
//! `SoundEnvelope` and `AudioFilter` (as documented by RuneLite,
//! BSD-2-Clause), keeping its integer arithmetic so the output matches.
//!
//! One deliberate difference: the reference client mixes instruments into
//! 8-bit samples; [`SoundEffect::render`] mixes the same 16-bit instrument
//! output into 16-bit samples instead.

use std::sync::OnceLock;

use crate::buffer::Buffer;
use crate::error::{Error, Result};

/// Output rate of every sound effect.
pub const SAMPLE_RATE: u32 = 22_050;

/// Cache index that holds sound effects; the group id is the sound id.
pub const SOUND_INDEX: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// Waveform for oscillators driven by this envelope: 1 square, 2 sine,
    /// 3 saw, 4 noise.
    pub form: u8,
    pub start: i32,
    pub end: i32,
    pub durations: Vec<u16>,
    pub phases: Vec<u16>,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            form: 0,
            start: 0,
            end: 0,
            durations: vec![0, 65535],
            phases: vec![0, 65535],
        }
    }
}

impl Envelope {
    fn decode(b: &mut Buffer<'_>) -> Result<Self> {
        let mut env = Self {
            form: b.read_u8()?,
            start: b.read_i32()?,
            end: b.read_i32()?,
            ..Self::default()
        };
        env.decode_segments(b)?;
        Ok(env)
    }

    fn decode_segments(&mut self, b: &mut Buffer<'_>) -> Result<()> {
        let count = b.read_u8()? as usize;
        self.durations = Vec::with_capacity(count);
        self.phases = Vec::with_capacity(count);
        for _ in 0..count {
            self.durations.push(b.read_u16()?);
            self.phases.push(b.read_u16()?);
        }
        Ok(())
    }
}

/// Playback state of one envelope (the reference client keeps it inside
/// the envelope object).
#[derive(Default)]
struct Stepper {
    ticks: i32,
    phase_index: usize,
    step: i32,
    amplitude: i32,
    max: i32,
}

impl Stepper {
    fn step(&mut self, env: &Envelope, period: i32) -> i32 {
        let segments = env.phases.len();
        if segments == 0 {
            return 0;
        }
        if self.max >= self.ticks {
            self.amplitude = (env.phases[self.phase_index] as i32) << 15;
            self.phase_index += 1;
            if self.phase_index >= segments {
                self.phase_index = segments - 1;
            }
            self.ticks = (env.durations[self.phase_index] as f64 / 65536.0 * period as f64) as i32;
            if self.ticks > self.max {
                self.step = (((env.phases[self.phase_index] as i32) << 15)
                    .wrapping_sub(self.amplitude))
                    / (self.ticks - self.max);
            }
        }
        self.amplitude = self.amplitude.wrapping_add(self.step);
        self.max += 1;
        self.amplitude.wrapping_sub(self.step) >> 15
    }
}

/// A two-direction IIR filter whose poles move along an envelope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    pub pairs: [usize; 2],
    pub phases: [[[u16; 4]; 2]; 2],
    pub magnitudes: [[[u16; 4]; 2]; 2],
    pub unity: [u16; 2],
}

/// Coefficients worked out by [`Filter::compute`].
struct Coefficients {
    values: [[i32; 8]; 2],
    forward_multiplier: i32,
}

impl Filter {
    fn decode(b: &mut Buffer<'_>, envelope: &mut Envelope) -> Result<Self> {
        let mut filter = Self::default();
        let packed = b.read_u8()?;
        filter.pairs = [(packed >> 4) as usize, (packed & 15) as usize];
        if packed == 0 {
            return Ok(filter);
        }
        if filter.pairs[0] > 4 || filter.pairs[1] > 4 {
            return Err(Error::parse(format!("sound filter: {packed:#x} pairs")));
        }
        filter.unity = [b.read_u16()?, b.read_u16()?];
        let migrated = b.read_u8()?;
        for dir in 0..2 {
            for pair in 0..filter.pairs[dir] {
                filter.phases[dir][0][pair] = b.read_u16()?;
                filter.magnitudes[dir][0][pair] = b.read_u16()?;
            }
        }
        for dir in 0..2 {
            for pair in 0..filter.pairs[dir] {
                if migrated & (1 << (dir * 4) << pair) != 0 {
                    filter.phases[dir][1][pair] = b.read_u16()?;
                    filter.magnitudes[dir][1][pair] = b.read_u16()?;
                } else {
                    filter.phases[dir][1][pair] = filter.phases[dir][0][pair];
                    filter.magnitudes[dir][1][pair] = filter.magnitudes[dir][0][pair];
                }
            }
        }
        if migrated != 0 || filter.unity[1] != filter.unity[0] {
            envelope.decode_segments(b)?;
        }
        Ok(filter)
    }

    fn magnitude(&self, dir: usize, pair: usize, f: f32) -> f32 {
        let [a, b] = [
            self.magnitudes[dir][0][pair] as f32,
            self.magnitudes[dir][1][pair] as f32,
        ];
        let db = (a + f * (b - a)) * 0.001_525_878_9;
        1.0 - 10f64.powf((-db / 20.0) as f64) as f32
    }

    fn phase(&self, dir: usize, pair: usize, f: f32) -> f32 {
        let [a, b] = [
            self.phases[dir][0][pair] as f32,
            self.phases[dir][1][pair] as f32,
        ];
        let octaves = (a + f * (b - a)) * 1.220_703_1e-4;
        let hz = 32.703_197 * 2f64.powf(octaves as f64) as f32;
        hz * std::f32::consts::PI / 11025.0
    }

    /// Fill `out` for direction `dir` at envelope position `f`; returns
    /// the number of coefficients.
    fn compute(&self, dir: usize, f: f32, out: &mut Coefficients) -> usize {
        let mut forward = 1.0f32;
        if dir == 0 {
            let [a, b] = [self.unity[0] as f32, self.unity[1] as f32];
            let db = (a + (b - a) * f) * 0.003_051_757_8;
            forward = 0.1f64.powf((db / 20.0) as f64) as f32;
            out.forward_multiplier = (forward * 65536.0) as i32;
        }
        let pairs = self.pairs[dir];
        if pairs == 0 {
            return 0;
        }
        let mut m = [0f32; 8];
        let r = self.magnitude(dir, 0, f);
        m[0] = -2.0 * r * (self.phase(dir, 0, f) as f64).cos() as f32;
        m[1] = r * r;
        for k in 1..pairs {
            let r = self.magnitude(dir, k, f);
            let a = -2.0 * r * (self.phase(dir, k, f) as f64).cos() as f32;
            let b = r * r;
            m[k * 2 + 1] = m[k * 2 - 1] * b;
            m[k * 2] = m[k * 2 - 1] * a + m[k * 2 - 2] * b;
            let mut j = k * 2 - 1;
            while j >= 2 {
                m[j] += m[j - 1] * a + m[j - 2] * b;
                j -= 1;
            }
            m[1] += m[0] * a + b;
            m[0] += a;
        }
        if dir == 0 {
            for value in m.iter_mut().take(pairs * 2) {
                *value *= forward;
            }
        }
        for (slot, value) in out.values[dir].iter_mut().zip(m).take(pairs * 2) {
            *slot = (value * 65536.0) as i32;
        }
        pairs * 2
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Oscillator {
    /// Percent of the volume envelope.
    pub volume: u16,
    /// Detune in steps of 1/120 octave.
    pub pitch: i32,
    /// Start delay in milliseconds.
    pub delay: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Instrument {
    pub pitch: Envelope,
    pub volume: Envelope,
    pub pitch_modifier: Option<(Envelope, Envelope)>,
    pub volume_modifier: Option<(Envelope, Envelope)>,
    /// Gate: release and attack envelopes.
    pub gate: Option<(Envelope, Envelope)>,
    pub oscillators: Vec<Oscillator>,
    pub delay_time: u16,
    pub delay_decay: u16,
    /// Length and start offset in milliseconds.
    pub duration: u16,
    pub offset: u16,
    pub filter: Filter,
    pub filter_envelope: Envelope,
}

fn optional_pair(b: &mut Buffer<'_>) -> Result<Option<(Envelope, Envelope)>> {
    if b.peek_u8()? == 0 {
        b.read_u8()?;
        return Ok(None);
    }
    Ok(Some((Envelope::decode(b)?, Envelope::decode(b)?)))
}

struct Tables {
    noise: Vec<i32>,
    sine: Vec<i32>,
}

/// The reference client's noise and sine tables. The noise comes from
/// `java.util.Random(0)`, reproduced here so the hiss matches.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        const MULT: u64 = 0x5_DEEC_E66D;
        const MASK: u64 = (1 << 48) - 1;
        let mut seed = MULT & MASK;
        let noise = (0..32768)
            .map(|_| {
                seed = (seed.wrapping_mul(MULT).wrapping_add(0xB)) & MASK;
                let next = (seed >> 16) as u32 as i32;
                (next & 2) - 1
            })
            .collect();
        let sine = (0..32768)
            .map(|i| ((i as f64 / 5215.1903).sin() * 16384.0) as i32)
            .collect();
        Tables { noise, sine }
    })
}

fn wave(phase: i32, amplitude: i32, form: u8) -> i32 {
    match form {
        1 => {
            if phase & 32767 < 16384 {
                amplitude
            } else {
                -amplitude
            }
        }
        2 => tables().sine[(phase & 32767) as usize].wrapping_mul(amplitude) >> 14,
        3 => (amplitude.wrapping_mul(phase & 32767) >> 14) - amplitude,
        4 => amplitude.wrapping_mul(tables().noise[((phase / 2607) & 32767) as usize]),
        _ => 0,
    }
}

fn mul16(sample: i32, coefficient: i32) -> i32 {
    ((sample as i64 * coefficient as i64) >> 16) as i32
}

impl Instrument {
    fn decode(b: &mut Buffer<'_>) -> Result<Self> {
        let pitch = Envelope::decode(b)?;
        let volume = Envelope::decode(b)?;
        let pitch_modifier = optional_pair(b)?;
        let volume_modifier = optional_pair(b)?;
        let gate = optional_pair(b)?;
        let mut oscillators = Vec::new();
        for _ in 0..10 {
            let volume = b.read_unsigned_short_smart()?;
            if volume == 0 {
                break;
            }
            oscillators.push(Oscillator {
                volume,
                pitch: b.read_short_smart()?,
                delay: b.read_unsigned_short_smart()?,
            });
        }
        let delay_time = b.read_unsigned_short_smart()?;
        let delay_decay = b.read_unsigned_short_smart()?;
        let duration = b.read_u16()?;
        let offset = b.read_u16()?;
        let mut filter_envelope = Envelope::default();
        let filter = Filter::decode(b, &mut filter_envelope)?;
        Ok(Self {
            pitch,
            volume,
            pitch_modifier,
            volume_modifier,
            gate,
            oscillators,
            delay_time,
            delay_decay,
            duration,
            offset,
            filter,
            filter_envelope,
        })
    }

    /// `count` samples of this instrument over `ms` milliseconds.
    pub fn synthesize(&self, count: usize, ms: u16) -> Vec<i32> {
        let mut samples = vec![0i32; count];
        if ms < 10 || count == 0 {
            return samples;
        }
        let n = count as i32;
        let rate = count as f64 / ms as f64;
        let mut pitch = Stepper::default();
        let mut volume = Stepper::default();

        let mut pitch_mod = (Stepper::default(), Stepper::default(), 0i32, 0i32, 0i32);
        if let Some((env, _)) = &self.pitch_modifier {
            pitch_mod.2 = ((env.end - env.start) as f64 * 32.768 / rate) as i32;
            pitch_mod.3 = (env.start as f64 * 32.768 / rate) as i32;
        }
        let mut volume_mod = (Stepper::default(), Stepper::default(), 0i32, 0i32, 0i32);
        if let Some((env, _)) = &self.volume_modifier {
            volume_mod.2 = ((env.end - env.start) as f64 * 32.768 / rate) as i32;
            volume_mod.3 = (env.start as f64 * 32.768 / rate) as i32;
        }

        struct Voice {
            phase: i32,
            delay: i32,
            volume_step: i32,
            pitch_step: i32,
            pitch_base: i32,
        }
        let mut voices: Vec<Voice> = self
            .oscillators
            .iter()
            .map(|osc| Voice {
                phase: 0,
                delay: (osc.delay as f64 * rate) as i32,
                volume_step: ((osc.volume as i32) << 14) / 100,
                pitch_step: ((self.pitch.end - self.pitch.start) as f64
                    * 32.768
                    * 1.005_792_941_067_853_4f64.powi(osc.pitch)
                    / rate) as i32,
                pitch_base: (self.pitch.start as f64 * 32.768 / rate) as i32,
            })
            .collect();

        for i in 0..n {
            let mut p = pitch.step(&self.pitch, n);
            let mut v = volume.step(&self.volume, n);
            if let Some((env, amp)) = &self.pitch_modifier {
                let step = pitch_mod.0.step(env, n);
                let amplitude = pitch_mod.1.step(amp, n);
                p = p.wrapping_add(wave(pitch_mod.4, amplitude, env.form) >> 1);
                pitch_mod.4 = pitch_mod
                    .4
                    .wrapping_add(pitch_mod.3)
                    .wrapping_add(step.wrapping_mul(pitch_mod.2) >> 16);
            }
            if let Some((env, amp)) = &self.volume_modifier {
                let step = volume_mod.0.step(env, n);
                let amplitude = volume_mod.1.step(amp, n);
                v = v.wrapping_mul((wave(volume_mod.4, amplitude, env.form) >> 1) + 32768) >> 15;
                volume_mod.4 = volume_mod
                    .4
                    .wrapping_add(volume_mod.3)
                    .wrapping_add(step.wrapping_mul(volume_mod.2) >> 16);
            }
            for voice in &mut voices {
                let at = voice.delay + i;
                if at < n {
                    let s = &mut samples[at as usize];
                    *s = s.wrapping_add(wave(
                        voice.phase,
                        v.wrapping_mul(voice.volume_step) >> 15,
                        self.pitch.form,
                    ));
                    voice.phase = voice
                        .phase
                        .wrapping_add(p.wrapping_mul(voice.pitch_step) >> 16)
                        .wrapping_add(voice.pitch_base);
                }
            }
        }

        if let Some((release, attack)) = &self.gate {
            let (mut r, mut a) = (Stepper::default(), Stepper::default());
            let mut counter = 0i32;
            let mut muted = true;
            for sample in samples.iter_mut() {
                let rs = r.step(release, n);
                let at = a.step(attack, n);
                let threshold = if muted {
                    (rs.wrapping_mul(release.end - release.start) >> 8) + release.start
                } else {
                    (at.wrapping_mul(release.end - release.start) >> 8) + release.start
                };
                counter += 256;
                if counter >= threshold {
                    counter = 0;
                    muted = !muted;
                }
                if muted {
                    *sample = 0;
                }
            }
        }

        if self.delay_time > 0 && self.delay_decay > 0 {
            let delay = (self.delay_time as f64 * rate) as usize;
            for i in delay..count {
                samples[i] = samples[i]
                    .wrapping_add(samples[i - delay].wrapping_mul(self.delay_decay as i32) / 100);
            }
        }

        if self.filter.pairs[0] > 0 || self.filter.pairs[1] > 0 {
            self.apply_filter(&mut samples);
        }

        for sample in &mut samples {
            *sample = (*sample).clamp(-32768, 32767);
        }
        samples
    }

    fn apply_filter(&self, samples: &mut [i32]) {
        let n = samples.len();
        let period = n as i32 + 1;
        let mut stepper = Stepper::default();
        let mut c = Coefficients {
            values: [[0; 8]; 2],
            forward_multiplier: 0,
        };
        let mut env = stepper.step(&self.filter_envelope, period);
        let mut forward = self.filter.compute(0, env as f32 / 65536.0, &mut c);
        let mut back = self.filter.compute(1, env as f32 / 65536.0, &mut c);
        if n < forward + back {
            return;
        }
        let mut i = 0usize;
        let first = back.min(n - forward);
        while i < first {
            let mut out = mul16(samples[i + forward], c.forward_multiplier);
            for k in 0..forward {
                out += mul16(samples[i + forward - 1 - k], c.values[0][k]);
            }
            for k in 0..i {
                out -= mul16(samples[i - 1 - k], c.values[1][k]);
            }
            samples[i] = out;
            env = stepper.step(&self.filter_envelope, period);
            i += 1;
        }
        let mut block = 128usize;
        loop {
            let end = block.min(n - forward);
            while i < end {
                let mut out = mul16(samples[i + forward], c.forward_multiplier);
                for k in 0..forward {
                    out += mul16(samples[i + forward - 1 - k], c.values[0][k]);
                }
                for k in 0..back {
                    out -= mul16(samples[i - 1 - k], c.values[1][k]);
                }
                samples[i] = out;
                env = stepper.step(&self.filter_envelope, period);
                i += 1;
            }
            if i >= n - forward {
                while i < n {
                    let mut out = 0;
                    for k in (i + forward - n)..forward {
                        out += mul16(samples[i + forward - 1 - k], c.values[0][k]);
                    }
                    for k in 0..back {
                        out -= mul16(samples[i - 1 - k], c.values[1][k]);
                    }
                    samples[i] = out;
                    stepper.step(&self.filter_envelope, period);
                    i += 1;
                }
                break;
            }
            forward = self.filter.compute(0, env as f32 / 65536.0, &mut c);
            back = self.filter.compute(1, env as f32 / 65536.0, &mut c);
            block += 128;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundEffect {
    pub id: u32,
    pub instruments: Vec<Instrument>,
    /// Loop points in milliseconds (used by looping area sounds).
    pub loop_start: u16,
    pub loop_end: u16,
}

impl SoundEffect {
    pub fn decode(id: u32, data: &[u8]) -> Result<Self> {
        let mut b = Buffer::new(data);
        let mut instruments = Vec::new();
        for _ in 0..10 {
            if b.peek_u8()? == 0 {
                b.read_u8()?;
                continue;
            }
            instruments.push(Instrument::decode(&mut b)?);
        }
        Ok(Self {
            id,
            instruments,
            loop_start: b.read_u16()?,
            loop_end: b.read_u16()?,
        })
    }

    /// Load sound `id` from the cache.
    pub fn load(cache: &crate::Cache, id: u32) -> Result<Self> {
        let data = cache.file(SOUND_INDEX, id, 0)?;
        Self::decode(id, &data)
    }

    /// Length in milliseconds: the end of the latest instrument.
    pub fn length_ms(&self) -> u32 {
        self.instruments
            .iter()
            .map(|i| i.duration as u32 + i.offset as u32)
            .max()
            .unwrap_or(0)
    }

    /// Mono 16-bit samples at [`SAMPLE_RATE`].
    pub fn render(&self) -> Vec<i16> {
        let total = self.length_ms() as usize * SAMPLE_RATE as usize / 1000;
        let mut mix = vec![0i32; total];
        for instrument in &self.instruments {
            let count = instrument.duration as usize * SAMPLE_RATE as usize / 1000;
            let offset = instrument.offset as usize * SAMPLE_RATE as usize / 1000;
            let samples = instrument.synthesize(count, instrument.duration);
            for (slot, sample) in mix[offset.min(total)..].iter_mut().zip(samples) {
                *slot += sample;
            }
        }
        mix.into_iter()
            .map(|s| s.clamp(-32768, 32767) as i16)
            .collect()
    }

    /// The rendered sound as a 16-bit mono WAV file.
    pub fn to_wav(&self) -> Vec<u8> {
        wav(&self.render(), SAMPLE_RATE)
    }
}

/// A 16-bit mono PCM WAV file.
pub fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(form: u8, start: i32, end: i32, segments: &[(u16, u16)]) -> Vec<u8> {
        let mut out = vec![form];
        out.extend_from_slice(&start.to_be_bytes());
        out.extend_from_slice(&end.to_be_bytes());
        out.push(segments.len() as u8);
        for (d, p) in segments {
            out.extend_from_slice(&d.to_be_bytes());
            out.extend_from_slice(&p.to_be_bytes());
        }
        out
    }

    /// One sine instrument: 100 ms at a fixed pitch, full volume.
    fn tone(filter: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(envelope(2, 100_000, 100_000, &[(0, 0), (65535, 0)]));
        data.extend(envelope(0, 0, 0, &[(0, 65535), (65535, 65535)]));
        data.extend([0, 0, 0]); // no pitch/volume modifier, no gate
        data.extend([100, 64, 0]); // oscillator: 100%, no detune, no delay
        data.push(0); // end of oscillators
        data.extend([0, 0]); // no echo
        data.extend(100u16.to_be_bytes());
        data.extend(0u16.to_be_bytes());
        data.extend_from_slice(filter);
        let mut sound = data;
        sound.extend([0; 9]); // nine empty instrument slots
        sound.extend([0, 0, 0, 0]); // loop points
        sound
    }

    #[test]
    fn java_random_zero_noise_matches_the_reference() {
        // java.util.Random(0).nextInt() starts -1155484576, -723955400.
        let mult = 0x5_DEEC_E66Du64;
        let mask = (1u64 << 48) - 1;
        let mut seed = mult & mask;
        seed = (seed.wrapping_mul(mult) + 0xB) & mask;
        assert_eq!((seed >> 16) as u32 as i32, -1155484576);
        seed = (seed.wrapping_mul(mult) + 0xB) & mask;
        assert_eq!((seed >> 16) as u32 as i32, -723955400);
        let noise = &tables().noise;
        assert_eq!(noise[0], (-1155484576 & 2) - 1);
        assert!(noise.iter().all(|&n| n == 1 || n == -1));
    }

    #[test]
    fn a_sine_tone_decodes_and_renders_at_full_length() {
        let sound = SoundEffect::decode(1, &tone(&[0])).expect("decode");
        assert_eq!(sound.instruments.len(), 1);
        assert_eq!(sound.length_ms(), 100);
        let samples = sound.render();
        assert_eq!(samples.len(), 2205);
        let peak = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > 8000, "audible: {peak}");
        // It crosses zero: a tone, not DC.
        assert!(samples.iter().any(|&s| s > 1000) && samples.iter().any(|&s| s < -1000));
        let wav = sound.to_wav();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 2205 * 2);
    }

    #[test]
    fn a_low_pass_filter_decodes_and_stays_bounded() {
        // One pole pair forward, fixed (no migration), unity 0.
        let mut filter = vec![0x10];
        filter.extend(0u16.to_be_bytes());
        filter.extend(0u16.to_be_bytes());
        filter.push(0);
        filter.extend(20_000u16.to_be_bytes());
        filter.extend(30_000u16.to_be_bytes());
        let sound = SoundEffect::decode(2, &tone(&filter)).expect("decode");
        assert_eq!(sound.instruments[0].filter.pairs, [1, 0]);
        let samples = sound.render();
        assert_eq!(samples.len(), 2205);
    }

    #[test]
    fn truncated_data_is_an_error_not_a_panic() {
        let data = tone(&[0]);
        for len in [0, 5, 20, data.len() - 3] {
            assert!(SoundEffect::decode(3, &data[..len]).is_err(), "len {len}");
        }
    }
}
