use core::panic;
use std::{fs::File, io::Write, sync::Mutex};

use chrono::{Datelike, Local};
use log::{Level, Log, Metadata, Record};

use super::{FormatArgs, translate_args};

pub enum RotateSize {
    Disabled,
    Enabled(usize),
}

pub enum RotateTime {
    Disabled,
    Day,
    Month,
    Year,
}

pub struct FileLogger {
    level: Level,
    formatter: Box<dyn Fn(FormatArgs) -> String + Send + Sync>,
    file_name: String,
    buf_size: usize,

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
        file_name: &str,
        buf_size: usize,
    ) -> Result<Box<dyn Log>, std::io::Error>
    where
        F: Fn(FormatArgs) -> String + Send + Sync + 'static,
    {
        if buf_size < 8 {
            panic!("buf_size < 8");
        }

        let (file, len) = Self::open(file_name)?;
        let state = FileLoggerState {
            file,
            len,
            write_buf: String::with_capacity(buf_size),
        };

        Ok(Box::new(Self {
            level,
            formatter: Box::new(formatter),
            file_name: file_name.to_string(),
            buf_size,
            state: Mutex::new(state),
        }))
    }

    fn open(file_name: &str) -> Result<(File, usize), std::io::Error> {
        let file = File::options().append(true).create(true).open(file_name)?;
        let len = file.metadata()?.len();

        Ok((file, len as usize))
    }

    /// Flush if needed, then write to write_buf
    fn buffered_write(&self, s: &str) {
        let mut state = self.state.lock().unwrap();
        let mut data = s;
        while !data.is_empty() {
            // buf capacity in bytes
            let rest = self.buf_size - state.write_buf.len();
            // copy as many bytes as possible, but it must end at char boundary
            let wsize = floor_char_booundary(data, rest);
            let wdata = &data[..wsize];
            data = &data[wsize..];
            if wdata.is_empty() {
                // write_buf full
                // flush to file
                self.flush_buf(&mut state);
            } else {
                // memcpy to write_buf
                state.write_buf.push_str(wdata);
            }
        }
    }

    /// Rotate if needed, then write write_buf to file
    fn flush_buf(&self, state: &mut FileLoggerState) {
        // if condition is met, close writer, open a new file
        // todo
        // write
        state.file.write_all(state.write_buf.as_bytes()).unwrap();
        state.write_buf.clear();
        todo!();
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
        let output = self.formatter.as_ref()(args);
        self.buffered_write(&output);
    }

    fn flush(&self) {
        let mut state = self.state.lock().unwrap();
        self.flush_buf(&mut state);
    }
}

// str::floor_char_boundary() is unstable yet
fn floor_char_booundary(s: &str, mut index: usize) -> usize {
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

fn now_ymd() -> (u32, u32, u32) {
    let now = Local::now();

    (now.year() as u32, now.month(), now.day())
}
