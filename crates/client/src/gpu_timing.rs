//! GPU pass timings from timestamp queries (docs/ENGINE_ROADMAP.md, P3).
//!
//! The timer writes the begin and end of every render pass into a query set,
//! resolves it into a small readback buffer on one frame in [`SAMPLE_FRAMES`],
//! and maps that buffer asynchronously. The CPU never waits for the GPU: a
//! later frame calls [`GpuTimer::poll`], reads the ticks and feeds the rolling
//! per-pass averages used by `--gpu-timing` and the benchmark line.
//!
//! Adapters without `TIMESTAMP_QUERY` simply do not create a timer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// One begin/end pair per render pass.
pub const SLOTS: usize = 9;
/// Pass names in query-slot order. Keep in step with the `mark` calls in
/// `Renderer::render`.
pub const PASS_NAMES: [&str; SLOTS] = [
    "shadow",
    "scene",
    "bloom bright",
    "bloom down",
    "bloom up",
    "ssao",
    "post",
    "hud",
    "interface",
];
/// How many frames between readbacks. Timestamps are written every frame;
/// only the resolve and map are this infrequent. A prime keeps the sample
/// from lining up with the shadow's every-other-frame cadence.
pub const SAMPLE_FRAMES: u32 = 61;

const QUERY_COUNT: u32 = (SLOTS as u32) * 2;
const STATE_IDLE: u8 = 0;
const STATE_READY: u8 = 1;
const STATE_FAILED: u8 = 2;

#[derive(Clone, Copy, Default)]
struct PassTotal {
    ms: f64,
    samples: u32,
}

pub struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    period_ns: f32,
    state: Arc<AtomicU8>,
    in_flight: bool,
    /// A readback copy was encoded and wants a map after the queue submit.
    map_pending: bool,
    /// Frames left until the next sample; zero means retry now.
    until_sample: u32,
    sampling: bool,
    /// Passes that wrote timestamps this frame.
    mask: u16,
    /// Passes that wrote timestamps in the frame being read back.
    pending_mask: u16,
    /// Sample generation, so a reset can discard a readback that was already
    /// in flight when the benchmark started.
    generation: u64,
    pending_generation: u64,
    samples_since_report: u32,
    totals: [PassTotal; SLOTS],
}

impl GpuTimer {
    /// Create the query set and buffers. The caller must have enabled
    /// `Features::TIMESTAMP_QUERY` on the device.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu pass timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: QUERY_COUNT,
        });
        let size = (QUERY_COUNT as u64) * 8;
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu pass timestamps resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu pass timestamps readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            query_set,
            resolve_buffer,
            readback_buffer,
            period_ns: queue.get_timestamp_period(),
            state: Arc::new(AtomicU8::new(STATE_IDLE)),
            in_flight: false,
            map_pending: false,
            until_sample: SAMPLE_FRAMES,
            sampling: false,
            mask: 0,
            pending_mask: 0,
            generation: 0,
            pending_generation: 0,
            samples_since_report: 0,
            totals: [PassTotal::default(); SLOTS],
        }
    }

    /// Start a new frame and decide whether this one is a sample frame.
    /// `shadow` says whether the sun shadow pass runs this frame; the scene,
    /// bloom, post and HUD passes always do. The interface slot is marked
    /// after the interface pass, which only exists on some configurations.
    pub fn begin_frame(&mut self, shadow: bool) {
        self.until_sample = self.until_sample.saturating_sub(1);
        // A readback in flight makes this frame wait one frame, not a whole
        // sample interval.
        self.sampling = self.until_sample == 0 && !self.in_flight;
        if self.sampling {
            self.until_sample = SAMPLE_FRAMES;
        }
        self.mask = if shadow { 1 } else { 0 } | 0b0111_1110;
    }

    /// Timestamp writes for one pass, on sample frames only.
    pub fn writes(&self, slot: usize) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        debug_assert!(slot < SLOTS);
        if !self.sampling {
            return None;
        }
        let base = slot as u32 * 2;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query_set,
            beginning_of_pass_write_index: Some(base),
            end_of_pass_write_index: Some(base + 1),
        })
    }

    /// Record that the pass at `slot` wrote its timestamps this frame.
    pub fn mark(&mut self, slot: usize) {
        debug_assert!(slot < SLOTS);
        if self.sampling {
            self.mask |= 1 << slot;
        }
    }

    /// Encode the resolve and copy for a sample frame. The map itself starts
    /// after the queue submit, otherwise the buffer is still mapped when the
    /// copy tries to use it. Does nothing on other frames.
    pub fn finish_frame(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.sampling {
            return;
        }
        self.sampling = false;
        self.in_flight = true;
        self.map_pending = true;
        self.pending_mask = self.mask;
        self.pending_generation = self.generation;
        let size = (QUERY_COUNT as u64) * 8;
        encoder.resolve_query_set(&self.query_set, 0..QUERY_COUNT, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &self.readback_buffer, 0, size);
    }

    /// Start the async map for the copy encoded by [`Self::finish_frame`].
    /// Call this after the queue submit.
    pub fn after_submit(&mut self) {
        if !self.map_pending {
            return;
        }
        self.map_pending = false;
        let state = self.state.clone();
        self.readback_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                state.store(
                    if result.is_ok() {
                        STATE_READY
                    } else {
                        STATE_FAILED
                    },
                    Ordering::Release,
                );
            });
    }

    /// Collect a completed readback without blocking. Call once per frame.
    pub fn poll(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        let state = self.state.swap(STATE_IDLE, Ordering::AcqRel);
        if state == STATE_FAILED {
            tracing::warn!("gpu timing: timestamp readback failed");
            self.in_flight = false;
            return;
        }
        if state != STATE_READY {
            return;
        }
        let mut ticks = [0u64; QUERY_COUNT as usize];
        {
            let Ok(view) = self.readback_buffer.slice(..).get_mapped_range() else {
                tracing::warn!("gpu timing: readback range was not mapped");
                self.in_flight = false;
                return;
            };
            for (tick, bytes) in ticks.iter_mut().zip(view.chunks_exact(8)) {
                *tick = u64::from_le_bytes(bytes.try_into().expect("eight bytes"));
            }
        }
        self.readback_buffer.unmap();
        self.in_flight = false;
        if self.period_ns <= 0.0 || self.pending_generation != self.generation {
            return;
        }
        for (slot, ms) in sample_pass_ms(&ticks, self.pending_mask, self.period_ns) {
            self.totals[slot].ms += ms;
            self.totals[slot].samples += 1;
        }
        self.samples_since_report += 1;
        if self.samples_since_report >= 5 {
            self.samples_since_report = 0;
            let detail = self
                .averages()
                .iter()
                .map(|(name, ms)| format!("{name}={ms:.2}ms"))
                .collect::<Vec<_>>()
                .join(" ");
            if let Some(total) = self.average_frame_ms() {
                tracing::info!("gpu passes: frame={total:.2}ms {detail}");
            }
        }
    }

    /// Rolling per-pass averages, in milliseconds, for passes that have a
    /// sample.
    pub fn averages(&self) -> Vec<(&'static str, f64)> {
        PASS_NAMES
            .iter()
            .zip(self.totals.iter())
            .filter(|(_, total)| total.samples > 0)
            .map(|(name, total)| (*name, total.ms / total.samples as f64))
            .collect()
    }

    /// Total measured GPU time for a frame, summed over passes with samples.
    pub fn average_frame_ms(&self) -> Option<f64> {
        let averages = self.averages();
        (!averages.is_empty()).then(|| averages.iter().map(|(_, ms)| ms).sum())
    }

    /// Forget the accumulated samples so a benchmark can measure one run.
    /// A readback already in flight is discarded when it arrives.
    pub fn reset(&mut self) {
        self.totals = [PassTotal::default(); SLOTS];
        self.generation = self.generation.wrapping_add(1);
    }
}

/// Turn a resolved query block into `(slot, milliseconds)` for passes that
/// both wrote timestamps and produced a sane interval.
pub fn sample_pass_ms(ticks: &[u64], mask: u16, period_ns: f32) -> Vec<(usize, f64)> {
    let mut samples = Vec::new();
    for slot in 0..SLOTS {
        if mask & (1 << slot) == 0 {
            continue;
        }
        let begin = ticks.get(slot * 2).copied().unwrap_or(0);
        let end = ticks.get(slot * 2 + 1).copied().unwrap_or(0);
        if end > begin && begin > 0 {
            samples.push((slot, (end - begin) as f64 * period_ns as f64 / 1_000_000.0));
        }
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_times_use_the_queue_period_and_mask() {
        // 1,000 ticks at 1 ns each is one microsecond (0.001 ms); 100 ticks
        // is a tenth of that.
        let mut ticks = [0u64; QUERY_COUNT as usize];
        ticks[0] = 10;
        ticks[1] = 1_010;
        ticks[2] = 5_000_000;
        ticks[3] = 7_000_000;
        ticks[12] = 100;
        ticks[13] = 200;
        let samples = sample_pass_ms(&ticks, 0b0100_0001, 1.0);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].0, 0);
        assert!((samples[0].1 - 0.001).abs() < 1e-12);
        assert_eq!(samples[1].0, 6);
        assert!((samples[1].1 - 0.000_1).abs() < 1e-15);

        let marked_off = sample_pass_ms(&ticks, 0, 1.0);
        assert!(marked_off.is_empty());
    }

    #[test]
    fn backwards_or_unwritten_intervals_are_ignored() {
        let mut ticks = [0u64; QUERY_COUNT as usize];
        ticks[0] = 900;
        ticks[1] = 800;
        ticks[2] = 0;
        ticks[3] = 500;
        ticks[4] = 100;
        ticks[5] = 400;
        let samples = sample_pass_ms(&ticks, 0b111, 1.0);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].0, 2);
        assert!((samples[0].1 - 0.000_3).abs() < 1e-12);
    }

    #[test]
    fn slot_names_stay_in_order() {
        assert_eq!(SLOTS, PASS_NAMES.len());
        assert_eq!(PASS_NAMES[0], "shadow");
        assert_eq!(PASS_NAMES[SLOTS - 1], "interface");
    }
}
