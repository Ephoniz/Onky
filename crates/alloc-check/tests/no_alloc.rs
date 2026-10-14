//! Snapshot a counting allocator around `Engine::process`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use audio_program_engine::{Engine, Frame, BLOCK_FRAMES, MAX_CHANNELS, SAMPLE_RATE};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn process_does_not_allocate() {
    let mut engine = Engine::new();
    for input in 0..4 {
        engine.configure_input(input, 2, SAMPLE_RATE);
        engine.shared().set_gain(input, 0.5);
        for output in 0..4 {
            engine.shared().set_route(input, output, true);
        }
    }
    for output in 0..4 {
        engine.configure_output(output, 2, SAMPLE_RATE, output == 0);
    }
    let mut frame: Frame = [0.0; MAX_CHANNELS];
    frame[0] = 0.2;
    frame[1] = -0.2;
    let block = vec![frame; 2_000];
    for input in 0..4 {
        engine.shared().push_input(input, &block);
    }
    for _ in 0..8 {
        engine.process();
        for input in 0..4 {
            engine.shared().push_input(input, &block[..BLOCK_FRAMES]);
        }
    }

    let before = ALLOCS.load(Ordering::Relaxed);
    engine.process();
    let after = ALLOCS.load(Ordering::Relaxed);
    assert_eq!(before, after, "process allocated {} times", after - before);
}
