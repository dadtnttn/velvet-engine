//! The GPU time of each pass, to find what is slow on a given GPU (QA: `VELVET_EXP=gputime`).
//!
//! Passes ask [`pass`] or [`compute_pass`] for their timestamp writes. Once a frame has been
//! submitted, [`end_frame`] resolves its timestamps into a small ring of buffers and reads back the
//! ones the GPU has finished with, a few frames late, so nothing waits for the GPU. Every 120 frames
//! it prints each pass's mean time (passes with the same name are added up) and the frame's span
//! on the GPU, from the first pass's start to the last one's end.
//!
//! Off (the default), every call does nothing and no pass is timed. The device needs
//! `Features::TIMESTAMP_QUERY`, which [`features`] asks for only when the timing is on. On a
//! tile-based GPU (phones) the passes of a frame overlap, so their times are a guide to where the
//! time goes rather than parts of a sum.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Timestamps a frame can hold: two a pass.
const QUERIES: u32 = 128;
/// Frames whose timestamps can be in flight at once.
const RING: usize = 4;
/// Frames averaged into each report.
const REPORT_EVERY: u32 = 120;

/// Whether the timing was asked for (`VELVET_EXP=gputime`).
pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var("VELVET_EXP").is_ok_and(|v| v.split(',').any(|f| f == "gputime"))
    })
}

/// The device features the timing needs: none when it is off or the adapter lacks them.
pub fn features(adapter: &wgpu::Adapter) -> wgpu::Features {
    if enabled() {
        adapter.features() & wgpu::Features::TIMESTAMP_QUERY
    } else {
        wgpu::Features::empty()
    }
}

/// The timestamp writes for a render pass called `label` (None when the timing is off, before its
/// first frame, or when this frame's timestamps are used up).
pub fn pass(label: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'static>> {
    let (query_set, first) = claim(label)?;
    Some(wgpu::RenderPassTimestampWrites {
        query_set,
        beginning_of_pass_write_index: Some(first),
        end_of_pass_write_index: Some(first + 1),
    })
}

/// [`pass`] for a compute pass.
pub fn compute_pass(label: &'static str) -> Option<wgpu::ComputePassTimestampWrites<'static>> {
    let (query_set, first) = claim(label)?;
    Some(wgpu::ComputePassTimestampWrites {
        query_set,
        beginning_of_pass_write_index: Some(first),
        end_of_pass_write_index: Some(first + 1),
    })
}

/// After a frame's last submission (its presentation): resolves the frame's timestamps, reads the
/// ones the GPU has finished, and prints a report every 120 frames.
pub fn end_frame(device: &wgpu::Device, queue: &wgpu::Queue) {
    if !enabled() || !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        return;
    }
    let Ok(mut guard) = TIMER.lock() else {
        return;
    };
    let timer = guard.get_or_insert_with(|| Timer::new(device, queue));
    timer.read_finished();
    // The copies submitted at the last frame's end can be mapped now.
    for slot in &mut timer.slots {
        if slot.state == SlotState::Submitted {
            let ready = slot.ready.clone();
            slot.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    if result.is_ok() {
                        ready.store(true, Ordering::Release);
                    }
                });
            slot.state = SlotState::Mapping;
        }
    }
    let count = timer.labels.len() as u32 * 2;
    if count > 0 {
        if let Some(slot) = timer.slots.iter_mut().find(|s| s.state == SlotState::Free) {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velvet-gpu-timer"),
            });
            encoder.resolve_query_set(timer.queries, 0..count, &timer.resolve, 0);
            encoder.copy_buffer_to_buffer(&timer.resolve, 0, &slot.buffer, 0, count as u64 * 8);
            queue.submit(Some(encoder.finish()));
            slot.labels = std::mem::take(&mut timer.labels);
            slot.state = SlotState::Submitted;
        }
    }
    timer.labels.clear();
}

static TIMER: Mutex<Option<Timer>> = Mutex::new(None);

fn claim(label: &'static str) -> Option<(&'static wgpu::QuerySet, u32)> {
    if !enabled() {
        return None;
    }
    let mut guard = TIMER.lock().ok()?;
    let timer = guard.as_mut()?;
    let first = timer.labels.len() as u32 * 2;
    if first + 2 > QUERIES {
        return None;
    }
    timer.labels.push(label);
    Some((timer.queries, first))
}

#[derive(PartialEq)]
enum SlotState {
    Free,
    Submitted,
    Mapping,
}

struct Slot {
    buffer: wgpu::Buffer,
    labels: Vec<&'static str>,
    state: SlotState,
    ready: Arc<AtomicBool>,
}

struct Timer {
    /// Kept for the whole run (only when the timing is on), so passes can borrow it freely.
    queries: &'static wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    /// Nanoseconds a timestamp tick.
    period_ns: f64,
    /// This frame's timed passes, in the order of their timestamp pairs.
    labels: Vec<&'static str>,
    /// Milliseconds summed over the report's frames, by pass name in first-seen order.
    totals: Vec<(&'static str, f64)>,
    span: f64,
    frames: u32,
}

impl Timer {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Timer {
        let queries: &'static wgpu::QuerySet =
            Box::leak(Box::new(device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("velvet-gpu-timer"),
                ty: wgpu::QueryType::Timestamp,
                count: QUERIES,
            })));
        let size = QUERIES as u64 * 8;
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-gpu-timer-resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..RING)
            .map(|_| Slot {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("velvet-gpu-timer-read"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                labels: Vec::new(),
                state: SlotState::Free,
                ready: Arc::new(AtomicBool::new(false)),
            })
            .collect();
        eprintln!(
            "[gputime] timing each pass ({} ns a tick)",
            queue.get_timestamp_period()
        );
        Timer {
            queries,
            resolve,
            slots,
            period_ns: queue.get_timestamp_period() as f64,
            labels: Vec::new(),
            totals: Vec::new(),
            span: 0.0,
            frames: 0,
        }
    }

    /// Adds up the slots the GPU has finished and frees them; prints when a report is full.
    fn read_finished(&mut self) {
        for i in 0..self.slots.len() {
            if self.slots[i].state != SlotState::Mapping
                || !self.slots[i].ready.load(Ordering::Acquire)
            {
                continue;
            }
            let labels = std::mem::take(&mut self.slots[i].labels);
            let mut first = u64::MAX;
            let mut last = 0u64;
            {
                let data = self.slots[i].buffer.slice(..).get_mapped_range();
                let stamps: &[u64] = bytemuck::cast_slice(&data);
                for (k, label) in labels.iter().enumerate() {
                    let (start, end) = (stamps[2 * k], stamps[2 * k + 1]);
                    if start == 0 || end <= start {
                        continue;
                    }
                    first = first.min(start);
                    last = last.max(end);
                    let ms = (end - start) as f64 * self.period_ns / 1.0e6;
                    match self.totals.iter_mut().find(|(name, _)| name == label) {
                        Some((_, total)) => *total += ms,
                        None => self.totals.push((label, ms)),
                    }
                }
            }
            self.slots[i].buffer.unmap();
            self.slots[i].ready.store(false, Ordering::Release);
            self.slots[i].state = SlotState::Free;
            if last > first {
                self.span += (last - first) as f64 * self.period_ns / 1.0e6;
            }
            self.frames += 1;
            if self.frames == REPORT_EVERY {
                let n = self.frames as f64;
                let parts: Vec<String> = self
                    .totals
                    .iter()
                    .map(|(name, total)| format!("{name} {:.2}", total / n))
                    .collect();
                let sum: f64 = self.totals.iter().map(|(_, total)| total / n).sum();
                eprintln!(
                    "[gputime] {} | sum {:.2}, span {:.2} ms ({} frames)",
                    parts.join(", "),
                    sum,
                    self.span / n,
                    self.frames
                );
                self.totals.clear();
                self.span = 0.0;
                self.frames = 0;
            }
        }
    }
}
