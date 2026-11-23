//! Sound effects: cache-synthesised OSRS sounds played through `rodio`.
//!
//! Gameplay code never touches audio. The scene queues [`SoundCue`]s (a
//! sound id, where it happened, volume and pitch) and the client drains
//! them once a frame into [`Audio::play_cues`], which fades them with
//! distance from the listener and pans them left or right of the camera.
//! Sounds are rendered from the cache the first time they are needed and
//! kept. Without an audio device, or with `--mute`, everything is a no-op.

use std::collections::HashMap;
use std::sync::Arc;

use glam::Vec3;
use rodio::Source;

/// One sound the scene wants played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundCue {
    pub id: u32,
    pub pos: Vec3,
    pub volume: f32,
    /// Playback rate: above 1 is higher and shorter.
    pub speed: f32,
}

/// Where the listener is: the camera's focus and its right-hand direction.
#[derive(Debug, Clone, Copy)]
pub struct Listener {
    pub pos: Vec3,
    pub right: Vec3,
}

/// Beyond this distance a sound is silent (world units; 20 tiles).
pub const HEARING_RANGE: f32 = 2_560.0;
/// Inside this distance a sound plays at full volume.
const FULL_VOLUME_RANGE: f32 = 384.0;
/// More sounds than this in one frame are dropped (a teamfight of hits
/// should not stack into a roar).
const MAX_CUES_PER_FRAME: usize = 8;

/// Volume and left/right gains for a sound at `pos`.
pub fn spatial_gains(listener: &Listener, pos: Vec3) -> Option<(f32, f32)> {
    let offset = Vec3::new(pos.x - listener.pos.x, 0.0, pos.z - listener.pos.z);
    let distance = offset.length();
    if distance >= HEARING_RANGE {
        return None;
    }
    let falloff = if distance <= FULL_VOLUME_RANGE {
        1.0
    } else {
        let t = (distance - FULL_VOLUME_RANGE) / (HEARING_RANGE - FULL_VOLUME_RANGE);
        (1.0 - t).powi(2)
    };
    // Constant-power pan, kept away from hard left/right.
    let pan = if distance > 1.0 {
        (offset.dot(listener.right) / distance).clamp(-1.0, 1.0) * 0.7
    } else {
        0.0
    };
    let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    Some((falloff * angle.cos(), falloff * angle.sin()))
}

/// Drop the silence some cache sounds start with (their instruments are
/// offset to line up with server ticks).
pub fn trim_leading_silence(samples: &[i16]) -> &[i16] {
    let start = samples
        .iter()
        .position(|s| s.unsigned_abs() > 64)
        .unwrap_or(samples.len());
    &samples[start..]
}

pub struct Audio {
    output: Option<(rodio::OutputStream, rodio::OutputStreamHandle)>,
    sounds: HashMap<u32, Option<Arc<Vec<i16>>>>,
    pub master: f32,
}

impl Audio {
    /// Open the default output device, or a silent stand-in.
    pub fn new(muted: bool) -> Self {
        let output = if muted {
            tracing::info!("audio: muted");
            None
        } else {
            match rodio::OutputStream::try_default() {
                Ok(output) => Some(output),
                Err(err) => {
                    tracing::warn!("audio: no output device ({err}); sound is off");
                    None
                }
            }
        };
        Self {
            output,
            sounds: HashMap::new(),
            master: 0.8,
        }
    }

    pub fn enabled(&self) -> bool {
        self.output.is_some()
    }

    /// Render these sounds now so the first play does not stall a frame.
    pub fn preload(&mut self, cache: &rift_cache::Cache, ids: impl IntoIterator<Item = u32>) {
        if !self.enabled() {
            return;
        }
        for id in ids {
            self.samples(cache, id);
        }
    }

    fn samples(&mut self, cache: &rift_cache::Cache, id: u32) -> Option<Arc<Vec<i16>>> {
        self.sounds
            .entry(id)
            .or_insert_with(|| match rift_cache::SoundEffect::load(cache, id) {
                Ok(sound) => {
                    let rendered = sound.render();
                    Some(Arc::new(trim_leading_silence(&rendered).to_vec()))
                }
                Err(err) => {
                    tracing::warn!("audio: sound {id} unavailable: {err:#}");
                    None
                }
            })
            .clone()
    }

    pub fn play_cues(
        &mut self,
        cache: &rift_cache::Cache,
        listener: &Listener,
        cues: impl IntoIterator<Item = SoundCue>,
    ) {
        if !self.enabled() {
            return;
        }
        for cue in cues.into_iter().take(MAX_CUES_PER_FRAME) {
            let Some((left, right)) = spatial_gains(listener, cue.pos) else {
                continue;
            };
            let Some(samples) = self.samples(cache, cue.id) else {
                continue;
            };
            tracing::debug!(id = cue.id, volume = cue.volume, "audio: cue");
            let gain = cue.volume * self.master;
            if gain * left.max(right) < 0.01 || samples.is_empty() {
                continue;
            }
            let stereo: Vec<i16> = samples
                .iter()
                .flat_map(|&s| {
                    let s = s as f32 * gain;
                    [(s * left) as i16, (s * right) as i16]
                })
                .collect();
            let source =
                rodio::buffer::SamplesBuffer::new(2, rift_cache::sound::SAMPLE_RATE, stereo)
                    .speed(cue.speed.clamp(0.25, 4.0));
            if let Some((_, handle)) = &self.output
                && let Err(err) = handle.play_raw(source.convert_samples())
            {
                tracing::warn!("audio: play failed: {err}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listener() -> Listener {
        Listener {
            pos: Vec3::ZERO,
            right: Vec3::X,
        }
    }

    #[test]
    fn near_sounds_are_full_and_far_sounds_are_silent() {
        let (l, r) = spatial_gains(&listener(), Vec3::new(0.0, 0.0, 100.0)).unwrap();
        assert!((l * l + r * r - 1.0).abs() < 0.01, "constant power");
        assert!((l - r).abs() < 0.01, "straight ahead is centred");
        assert!(spatial_gains(&listener(), Vec3::new(0.0, 0.0, HEARING_RANGE)).is_none());
        let (fl, fr) = spatial_gains(&listener(), Vec3::new(0.0, 0.0, 1800.0)).unwrap();
        assert!(fl < l && fr < r);
    }

    #[test]
    fn sounds_on_the_right_play_louder_on_the_right() {
        let (l, r) = spatial_gains(&listener(), Vec3::new(600.0, 0.0, 0.0)).unwrap();
        assert!(r > l * 2.0, "{l} {r}");
        assert!(l > 0.0, "never hard-panned");
        let (l, r) = spatial_gains(&listener(), Vec3::new(-600.0, 0.0, 0.0)).unwrap();
        assert!(l > r * 2.0);
    }

    #[test]
    fn leading_silence_is_trimmed() {
        let samples = [0i16, 3, -10, 0, 900, -800, 0];
        assert_eq!(trim_leading_silence(&samples), &[900, -800, 0]);
        assert!(trim_leading_silence(&[0, 0]).is_empty());
    }
}
