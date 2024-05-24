use std::{
    io::{Stdout, Write},
    sync::OnceLock,
    time::Duration,
};

use compact_str::{format_compact, CompactString};
use crossterm::{cursor, style::Print, QueueableCommand};

pub(crate) fn stdout() -> &'static mut Stdout {
    static mut STDOUT: OnceLock<Stdout> = OnceLock::new();
    unsafe {
        match STDOUT.get_mut() {
            Some(out) => out,
            None => {
                STDOUT.set(std::io::stdout()).unwrap();
                STDOUT.get_mut().unwrap()
            }
        }
    }
}

pub fn progress_bar<S, E, F>(
    mut handler: F,
    show_percent: bool,
    progress_len: u16,
    render_dur: Duration,
    title: Option<S>,
    fill_symbol: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>>
where
    S: AsRef<str>,
    E: std::error::Error + 'static,
    F: FnMut() -> Result<f64, E>,
{
    let mut current_progress = handler()?;
    if current_progress > 1.0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Progress data is out of 1!",
        )
        .into());
    }
    const PERCENT_VALUE_RESET: &str = "  0";
    const PERCENT_FIXED_SIZE: u16 = 9;
    const PERCENT_VALUE_HEAD: &str = "[ ";
    const PERCENT_VALUE_TAIL: &str = "% ]";
    let mut percent_start_col = 0;
    let mut rendered_len: u16 = 0;
    let mut head_size = 0;

    stdout()
        .queue(cursor::MoveToNextLine(1))
        .unwrap()
        .queue(cursor::Hide)
        .unwrap();
    if let Some(t) = title {
        let s = t.as_ref();
        let title_size = s.len() as u16 + 2;
        head_size += title_size;
        stdout().queue(Print(format_args!("{}: ", s))).unwrap();
    }
    if show_percent {
        head_size += PERCENT_FIXED_SIZE;
        percent_start_col =
            head_size - PERCENT_VALUE_RESET.len() as u16 - PERCENT_VALUE_TAIL.len() as u16 - 1;
        stdout()
            .queue(Print(format_args!(
                "{}{}{} ",
                PERCENT_VALUE_HEAD, PERCENT_VALUE_RESET, PERCENT_VALUE_TAIL
            )))
            .unwrap();
    }
    let num_screen_cols = crossterm::terminal::size().unwrap().0;
    let max_progress_len = if progress_len == 0 {
        num_screen_cols - head_size - 1
    } else {
        std::cmp::min(progress_len, num_screen_cols - head_size - 1)
    };

    stdout()
        .queue(Print(repeat_byte(b'-', max_progress_len)))
        .unwrap()
        .flush()
        .unwrap();

    let symbol = if let Some(c) = fill_symbol { c } else { b'#' };

    while current_progress <= 1.0 {
        if show_percent {
            let percent = format_compact!("{}", (current_progress * 100.0) as u16);
            stdout()
                .queue(cursor::MoveToColumn(percent_start_col))
                .unwrap()
                .queue(Print(PERCENT_VALUE_RESET))
                .unwrap()
                .queue(cursor::MoveToColumn(
                    percent_start_col + 3 - percent.len() as u16,
                ))
                .unwrap()
                .queue(Print(percent))
                .unwrap();
        }
        let curr_len = (current_progress * max_progress_len as f64) as u16;
        stdout()
            .queue(cursor::MoveToColumn(rendered_len + head_size))
            .unwrap()
            .queue(Print(repeat_byte(symbol, curr_len - rendered_len)))
            .unwrap()
            .flush()
            .unwrap();
        rendered_len = curr_len;
        std::thread::sleep(render_dur);
        current_progress = handler()?;
    }
    stdout()
        .queue(cursor::MoveToNextLine(2))
        .unwrap()
        .queue(cursor::Show)
        .unwrap()
        .flush()
        .unwrap();
    Ok(())
}

pub(crate) fn repeat_byte(ch: u8, times: u16) -> CompactString {
    let bytes: Vec<u8> = std::iter::repeat(ch).take(times as usize).collect();
    unsafe { CompactString::from_utf8_unchecked(bytes) }
}
