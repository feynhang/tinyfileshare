use std::time::Duration;

use crossterm::event;

static mut RUNNING: bool = false;

fn running() -> bool {
    unsafe { RUNNING }
}

fn set_running(state: bool) {
    unsafe { RUNNING = state };
}

#[derive(Debug)]
pub struct SimpleErr;
impl std::fmt::Display for SimpleErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}
impl std::error::Error for SimpleErr {}

fn main() {
    ctrlc::set_handler(|| set_running(false)).expect("set ctrlc handler failed!");
    std::thread::spawn(|| {
        while running() {
            if let Ok(event::Event::Key(event::KeyEvent {
                code: event::KeyCode::Esc,
                ..
            })) = crossterm::event::read()
            {
                set_running(false);
            } else {
                std::thread::yield_now();
            }
        }
    });

    let mut i = 0.0;
    let size = 100.0;
    set_running(true);
    if let Err(e) = termrender::progress_bar(
        || {
            if !running() {
                Err(SimpleErr)
            } else {
                i += 1.0;
                Ok(i / size)
            }
        },
        true,
        120,
        Duration::from_millis(25),
        Some("downloading"),
        Some(b'|'),
    ) {
        println!("{}", e)
    }
}
