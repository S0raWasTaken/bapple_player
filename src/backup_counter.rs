// In case there is no audio to sync the frames to,
// we'll use the old wall clock approach for syncing.
// It is not perfect, but hey, there's no audio to desync with!

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    thread::sleep,
    time::Duration,
};

// Only 1 reader and 1 writer at once. Desync happens, but it's acceptable in this scenario.
pub static SYNC_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn outside_counter(frametime: Duration, length: usize) {
    let mut counter = 0;
    while counter < length {
        sleep(frametime);
        counter += 1;
        SYNC_COUNTER.store(counter, Ordering::Relaxed);
    }
}
