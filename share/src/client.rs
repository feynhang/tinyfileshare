use std::{collections::VecDeque, path::PathBuf};

pub struct Client {
    paths: VecDeque<PathBuf>,
    ignore_dir: bool,
}

impl Client {
    fn checked_path(path_str: String) -> Option<PathBuf> {
        let path_buf = PathBuf::from(path_str);
        if path_buf.is_file() {
            Some(path_buf)
        } else {
            None
        }
    }

    pub fn new() -> Self {
        todo!()
    }
}
