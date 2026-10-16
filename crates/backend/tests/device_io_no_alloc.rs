//! Count allocations around the work a device thread does for each packet.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use audio_program_backend::{fill_render, mix_ahead, push_capture, DEVICE_CHANNELS};
use audio_program_engine::{Engine, SAMPLE_RATE};

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
fn device_packets_do_not_allocate() {
    let mut engine = Engine::new();
    let shared = engine.shared();
    engine.configure_input(0, DEVICE_CHANNELS, 44_100);
    engine.configure_output(0, DEVICE_CHANNELS, SAMPLE_RATE, true);
    engine.configure_output(1, DEVICE_CHANNELS, 44_100, false);
    shared.set_route(0, 0, true);
    shared.set_route(0, 1, true);
    let capture: Vec<u8> = (0..441 * DEVICE_CHANNELS)
        .flat_map(|index| ((index as f32) * 1e-3).to_le_bytes())
        .collect();
    let mut render = vec![0u8; 1056 * DEVICE_CHANNELS * 4];

    let before = ALLOCS.load(Ordering::Relaxed);
    for _ in 0..20 {
        push_capture(&shared, 0, &capture, DEVICE_CHANNELS, false);
        push_capture(&shared, 0, &capture, DEVICE_CHANNELS, true);
        mix_ahead(&mut engine, &shared, 0, 1056);
        fill_render(&shared, 0, &mut render, DEVICE_CHANNELS);
        fill_render(
            &shared,
            1,
            &mut render[..441 * DEVICE_CHANNELS * 4],
            DEVICE_CHANNELS,
        );
    }
    let after = ALLOCS.load(Ordering::Relaxed);
    assert_eq!(
        before,
        after,
        "device packets allocated {} times",
        after - before
    );
}
