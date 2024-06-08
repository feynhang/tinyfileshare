use std::path::PathBuf;

use crate::{error::CommonError, CommonResult};

pub const KB: usize = 1024;
pub const MB: usize = usize::pow(KB, 2);
pub const GB: usize = usize::pow(MB, 2);

// pub trait BytesExtensions {
//     fn trim(&self) -> &[u8];
//     fn trim_start(&self) -> &[u8];
//     fn trim_end(&self) -> &[u8];
//     fn find_first(&self, byte: u8) -> Option<usize>;
//     fn rfind_first(&self, byte: u8) -> Option<usize>;
//     fn nth_line(&self, n: usize) -> Option<&[u8]>;
//     fn skip_lines(&self, num_lines: usize) -> Option<&[u8]>;
//     // fn next_line(&self) -> Option<&[u8]>;
// }

// impl BytesExtensions for Vec<u8> {
//     fn trim(&self) -> &[u8] {
//         self.as_slice().trim()
//     }

//     fn trim_start(&self) -> &[u8] {
//         self.as_slice().trim_start()
//     }

//     fn trim_end(&self) -> &[u8] {
//         self.as_slice().trim_end()
//     }

//     fn find_first(&self, byte: u8) -> Option<usize> {
//         self.as_slice().find_first(byte)
//     }

//     fn rfind_first(&self, byte: u8) -> Option<usize> {
//         self.as_slice().rfind_first(byte)
//     }

//     fn nth_line(&self, n: usize) -> Option<&[u8]> {
//         self.as_slice().nth_line(n)
//     }

//     fn skip_lines(&self, num: usize) -> Option<&[u8]> {
//         self.as_slice().skip_lines(num)
//     }

//     // fn next_line(&self) -> Option<&[u8]> {
//     //     self.as_slice().next_line()
//     // }
// }

// impl BytesExtensions for [u8] {
//     fn trim(&self) -> &[u8] {
//         self.trim_start().trim_end()
//     }

//     fn trim_start(&self) -> &[u8] {
//         let mut head = 0;
//         while let Some(b) = self.iter().next() {
//             match *b {
//                 b' ' | b'\x09'..=b'\x0d' => break,
//                 _ => head += 1,
//             }
//         }
//         &self[head..]
//     }

//     fn trim_end(&self) -> &[u8] {
//         let mut tail = self.len();
//         while let Some(b) = self.iter().next_back() {
//             match *b {
//                 b' ' | b'\x09'..=b'\x0d' => break,
//                 _ => tail -= 1,
//             }
//         }
//         &self[0..tail]
//     }

//     fn find_first(&self, byte: u8) -> Option<usize> {
//         let mut idx = 0;
//         while let Some(b) = self.iter().next() {
//             if *b == byte {
//                 return Some(idx);
//             }
//             idx += 1;
//         }
//         None
//     }

//     fn rfind_first(&self, byte: u8) -> Option<usize> {
//         let mut idx = self.len() - 1;
//         while let Some(b) = self.iter().next_back() {
//             if *b == byte {
//                 return Some(idx);
//             }
//             idx -= 1;
//         }
//         None
//     }

//     fn nth_line(&self, n: usize) -> Option<&[u8]> {
//         let mut lines_count = 0;
//         let mut i = 0;
//         loop {
//             if let Some(newline_idx) = self[i..].find_first(b'\n') {
//                 if lines_count == n {
//                     return Some(&self[i..=newline_idx]);
//                 }
//                 if newline_idx + 1 > self.len() - 1 {
//                     break;
//                 }
//                 i = newline_idx + 1;
//                 lines_count += 1;
//             }
//         }
//         None
//     }

//     fn skip_lines(&self, num: usize) -> Option<&[u8]> {
//         let mut lines_count = 0;
//         let mut i = 0;
//         while i < self.len() {
//             if let Some(newline_idx) = self[i..].find_first(b'\n') {
//                 if lines_count == num - 1 {
//                     return Some(&self[i..]);
//                 }
//                 i = newline_idx + 1;
//                 lines_count += 1;
//             }
//         }
//         None
//     }


// }
