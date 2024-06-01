use std::thread::{self, JoinHandle};

use crossbeam::channel::Receiver;

use crate::{global, handler::Handler};

#[derive(Debug)]
pub struct Workers {
    _inner_handles: Vec<JoinHandle<()>>,
}

impl Workers {
    pub(crate) fn start(handler_rx: Receiver<Handler>) -> Self {
        let mut index = 0;
        let mut handles = vec![];
        while index <= global::config().num_workers() as usize {
            let rx_in = handler_rx.clone();
            // let tx_in = result_tx.clone();
            handles[index] = thread::spawn(move || Self::work_loop(rx_in));
            index += 1;
        }
        Self {
            _inner_handles: handles,
        }
    }

    pub(crate) fn work_loop(handler_rx: Receiver<Handler>) {
        loop {
            if let Ok(handler) = handler_rx.recv() {
                if let Err(e) = handler.handle() {
                    global::logger().log(format_args!("Error occurred: {}", e), crate::log::LogLevel::Error);
                }
                // result_tx.send(result).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;

    #[derive(Debug, PartialEq)]
    struct Example(i32, i32);

    impl Example {
        fn set_value(&mut self) {
            let mut_p_v0 = ptr::addr_of_mut!(self.0);
            let mut_p_v1 = ptr::addr_of_mut!(self.1);
            mod_v1(mut_p_v1, 4096);
            print_v1(mut_p_v1);
            mod_v1(mut_p_v0, 2048);
            print_v1(mut_p_v0);
        }
    }

    fn print_v1(p_v1: *const i32) {
        println!("v1 = {}", unsafe { *p_v1 })
    }

    fn mod_v1(p_v1: *mut i32, new_v: i32) {
        unsafe { *p_v1 = new_v }
    }

    #[test]
    fn unsafe_test() {
            let mut v1 = 1020;
            let mut_p_v1 = ptr::addr_of_mut!(v1);
            let mut_p2_v1 = ptr::addr_of_mut!(v1);
            
            mod_v1(mut_p_v1, 150);
            assert_eq!(v1, 150);
            print_v1(&v1);
            
            mod_v1(mut_p2_v1, 2560);
            assert_eq!(v1, 2560);
            print_v1(&v1);
    }

    #[test]
    fn scoped_addr_test() {
        let mut e = Example(10, 20);
        e.set_value();
        assert_eq!(e, Example(2048, 4096));
    }
}
