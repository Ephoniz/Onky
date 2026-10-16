use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Frame, RING_FRAMES};

const MASK: usize = RING_FRAMES - 1;

/// Single-producer, single-consumer frame queue.
///
/// One slot stays empty so the counters can tell full from empty. Indices are
/// atomics; the audio callback never locks this. Frame slots use interior
/// mutability because the producer and consumer each hold `&self`.
pub struct SpscRing {
    buf: Box<[UnsafeCell<Frame>]>,
    write: AtomicUsize,
    read: AtomicUsize,
}

// The producer writes a slot only before publishing it with the write counter.
// The consumer reads a slot only after observing that counter. Those two sides
// never touch the same slot at the same time.
unsafe impl Send for SpscRing {}
unsafe impl Sync for SpscRing {}

impl SpscRing {
    pub fn new() -> Self {
        Self {
            buf: (0..RING_FRAMES)
                .map(|_| UnsafeCell::new([0.0; crate::MAX_CHANNELS]))
                .collect(),
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    pub fn available(&self) -> usize {
        let write = self.write.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Acquire);
        write.wrapping_sub(read)
    }

    /// Copy `frames` into the ring. Returns how many were stored.
    pub fn push(&self, frames: &[Frame]) -> usize {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let used = write.wrapping_sub(read);
        let free = RING_FRAMES - 1 - used;
        let count = frames.len().min(free);
        for (offset, frame) in frames.iter().take(count).enumerate() {
            let index = (write.wrapping_add(offset)) & MASK;
            unsafe {
                *self.buf[index].get() = *frame;
            }
        }
        self.write
            .store(write.wrapping_add(count), Ordering::Release);
        count
    }

    pub fn push_frame(&self, frame: &Frame) -> bool {
        self.push(std::slice::from_ref(frame)) == 1
    }

    /// Copy up to `out.len()` frames into `out`. Returns how many were read.
    pub fn pop(&self, out: &mut [Frame]) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        let used = write.wrapping_sub(read);
        let count = out.len().min(used);
        for (offset, slot) in out.iter_mut().take(count).enumerate() {
            let index = (read.wrapping_add(offset)) & MASK;
            unsafe {
                *slot = *self.buf[index].get();
            }
        }
        self.read.store(read.wrapping_add(count), Ordering::Release);
        count
    }
}

#[cfg(test)]
#[path = "ring_tests.rs"]
mod tests;
