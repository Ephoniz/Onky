use super::*;
use crate::MAX_CHANNELS;

fn frame(n: usize) -> Frame {
    let mut frame = [0.0; MAX_CHANNELS];
    frame[0] = n as f32;
    frame
}

#[test]
fn ring_stops_at_one_free_slot_and_wraps() {
    let ring = SpscRing::new();
    let frames: Vec<Frame> = (0..RING_FRAMES).map(frame).collect();
    assert_eq!(ring.push(&frames), RING_FRAMES - 1);
    assert!(!ring.push_frame(&frame(0)));
    assert_eq!(ring.available(), RING_FRAMES - 1);

    let mut first = vec![[0.0; MAX_CHANNELS]; 10];
    assert_eq!(ring.pop(&mut first), 10);
    for (index, sample) in first.iter().enumerate() {
        assert_eq!(sample[0], index as f32);
    }

    let extra: Vec<Frame> = (0..10).map(|index| frame(1_000 + index)).collect();
    assert_eq!(ring.push(&extra), 10);

    let mut rest = vec![[0.0; MAX_CHANNELS]; RING_FRAMES];
    let got = ring.pop(&mut rest);
    assert_eq!(got, RING_FRAMES - 1);
    assert_eq!(rest[0][0], 10.0);
    assert_eq!(rest[got - 10][0], 1_000.0);
    assert_eq!(rest[got - 1][0], 1_009.0);
    assert_eq!(ring.available(), 0);
    assert_eq!(ring.pop(&mut rest), 0);
}
