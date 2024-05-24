use std::time::Duration;

use crossterm::event;

static mut RUNNING: bool = false;

fn running() -> bool {
    unsafe { RUNNING }
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
    ctrlc::set_handler(|| unsafe { RUNNING = false }).expect("set ctrlc handler failed!");
    std::thread::spawn(|| {
        while running() {
            let res = crossterm::event::read();
            if res.is_err() {
                continue;
            }
            match res.unwrap() {
                event::Event::Key(k_evt) => match k_evt {
                    event::KeyEvent {
                        code: event::KeyCode::Esc,
                        ..
                    } => unsafe { RUNNING = false },
                    _ => (),
                },
                _ => (),
            }
        }
    });

    let mut i = 0.0;
    let size = 100.0;
    unsafe {
        RUNNING = true;
    }
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
