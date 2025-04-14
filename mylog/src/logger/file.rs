use super::{FormatArgs, translate_args};

use chrono::{Datelike, Local};
use core::panic;
use log::{Level, Log, Metadata, Record};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug, Clone, Default)]
pub struct RotateOptions {
    pub size: RotateSize,
    pub time: RotateTime,
    pub file_count: u16,
}

#[derive(Debug, Clone, Default)]
pub enum RotateSize {
    #[default]
    Disabled,
    Enabled(usize),
}

#[derive(Debug, Clone, Default)]
pub enum RotateTime {
    #[default]
    Disabled,
    /// For debug
    Second,
    Day,
    Month,
    Year,
}

pub struct FileLogger {
    level: Level,
    formatter: Box<dyn Fn(FormatArgs) -> String + Send + Sync>,
    file_path: PathBuf,
    buf_size: usize,
    rotate_opts: RotateOptions,

    state: Mutex<FileLoggerState>,
}

struct FileLoggerState {
    file: File,
    len: usize,
    write_buf: String,
}

impl FileLogger {
    pub fn new<F>(
        level: Level,
        formatter: F,
        file_path: impl AsRef<Path>,
        buf_size: usize,
        rotate_opts: RotateOptions,
    ) -> Result<Box<dyn Log>, std::io::Error>
    where
        F: Fn(FormatArgs) -> String + Send + Sync + 'static,
    {
        if buf_size < 8 {
            panic!("buf_size < 8");
        }

        let (file, len) = Self::open(file_path.as_ref())?;
        let state = FileLoggerState {
            file,
            len,
            write_buf: String::with_capacity(buf_size),
        };

        Ok(Box::new(Self {
            level,
            formatter: Box::new(formatter),
            file_path: file_path.as_ref().into(),
            buf_size,
            rotate_opts,
            state: Mutex::new(state),
        }))
    }

    fn open(file_path: impl AsRef<Path>) -> Result<(File, usize), std::io::Error> {
        let file = File::options().append(true).create(true).open(file_path)?;
        let len = file.metadata()?.len();

        Ok((file, len as usize))
    }

    /// Flush if needed, then write to write_buf
    fn buffered_write_entry(&self, log_entry_str: &str) {
        // lock
        let mut state = self.state.lock().unwrap();

        // rotate check
        let mut rotate = false;
        if let RotateSize::Enabled(size) = self.rotate_opts.size {
            // if it would exceed the limit, rotate before write
            // if log_entry_str is longer than the limit, rotate and write it.
            if state.len.saturating_add(log_entry_str.len()) > size {
                rotate = true;
            }
        }
        match self.rotate_opts.time {
            //RotateTime::Day
            _ => {}
        }
        if rotate {
            self.rotate();
        }

        let mut data = log_entry_str;
        while !data.is_empty() {
            // buf capacity in bytes
            let rest = self.buf_size - state.write_buf.len();
            // copy as many bytes as possible, but it must end at char boundary
            let wsize = floor_char_boundary(data, rest);
            let wdata = &data[..wsize];
            data = &data[wsize..];
            if wdata.is_empty() {
                // write_buf full
                // flush to file
                self.flush_buf(&mut state);
                debug_assert!(state.write_buf.is_empty());
            } else {
                // memcpy to write_buf
                state.write_buf.push_str(wdata);
                state.len += wdata.len();
            }
        }
    }

    /// Called when buffer becomes full and when log::flush() is called
    fn flush_buf(&self, state: &mut FileLoggerState) {
        // write
        state.file.write_all(state.write_buf.as_bytes()).unwrap();
        state.write_buf.clear();
    }

    fn rotate(&self, state: &mut FileLoggerState) {
        std::fs::rename(from, to)
        Self::open(self.file_path);
        state.file
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = Local::now();
        let args = translate_args(record, timestamp);
        let mut output = self.formatter.as_ref()(args);
        output.push('\n');
        self.buffered_write_entry(&output);
    }

    fn flush(&self) {
        let mut state = self.state.lock().unwrap();
        self.flush_buf(&mut state);
    }
}

// str::floor_char_boundary() is unstable yet
fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        loop {
            if s.is_char_boundary(index) {
                break;
            } else {
                index -= 1;
            }
        }
        index
    }
}

fn now_ymd() -> (u32, u32, u32, i64) {
    let now = Local::now();

    (now.year() as u32, now.month(), now.day(), now.timestamp())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn char_boundary() {
        let org = "abcde";
        let r = floor_char_boundary(org, 100);
        assert_eq!(org, &org[..r]);

        let org = "あいうえお";
        let r = floor_char_boundary(org, 5);
        assert_eq!(&org[..r], "あ");
        let r = floor_char_boundary(org, 1);
        assert_eq!(&org[..r], "");
    }
}
