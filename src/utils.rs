use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct ConsoleTimer(JoinHandle<()>, Arc<AtomicBool>);

pub fn start_console_timer() -> ConsoleTimer {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let handle = thread::spawn(move || {
        let start = Instant::now();

        while running_clone.load(std::sync::atomic::Ordering::Relaxed) {
            let elapsed = start.elapsed();
            print!(
                "\rElapsed: {:02}:{:02}:{:02}",
                elapsed.as_secs() / 3600,
                (elapsed.as_secs() / 60) % 60,
                elapsed.as_secs() % 60
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });
    ConsoleTimer(handle, running)
}

pub fn stop_console_timer(timer: ConsoleTimer) {
    let ConsoleTimer(handle, running) = timer;
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    handle.join().unwrap();
}
