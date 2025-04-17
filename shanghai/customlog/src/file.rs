use super::{FormatArgs, translate_args};

use chrono::{DateTime, Datelike, Local};
use core::panic;
use log::{LevelFilter, Log, Metadata, Record};
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
    level: LevelFilter,
    formatter: Box<dyn Fn(FormatArgs) -> String + Send + Sync>,
    /// Absolute path to the main log file
    file_path: PathBuf,
    dir_path: PathBuf,
    file_name: String,
    buf_size: usize,
    rotate_opts: RotateOptions,

    /// Lock for log
    state: Mutex<FileLoggerState>,
}

struct FileLoggerState {
    file: File,
    len: usize,
    write_buf: String,
    last_update: Option<(i64, u32, u32, u32)>,
}

impl FileLogger {
    pub fn new_boxed<F>(
        level: LevelFilter,
        formatter: F,
        file_path: impl AsRef<Path>,
        buf_size: usize,
        rotate_opts: RotateOptions,
    ) -> Result<Box<dyn Log>, anyhow::Error>
    where
        F: Fn(FormatArgs) -> String + Send + Sync + 'static,
    {
        if buf_size < 8 {
            panic!("buf_size < 8");
        }

        let (file, len) = open_new_or_append(&file_path)?;
        let state = FileLoggerState {
            file,
            len,
            write_buf: String::with_capacity(buf_size),
            last_update: None,
        };

        let file_path = file_path.as_ref().canonicalize()?;
        let dir_path = file_path.parent().unwrap().to_path_buf();
        let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();

        Ok(Box::new(Self {
            level,
            formatter: Box::new(formatter),
            file_path,
            dir_path,
            file_name,
            buf_size,
            rotate_opts,
            state: Mutex::new(state),
        }))
    }

    /// Flush if needed, then write to write_buf
    fn buffered_write_entry(&self, ts: &DateTime<Local>, log_entry_str: &str) {
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
        if let Some((lsec, ly, lm, ld)) = state.last_update {
            let (sec, y, m, d) = to_ymd(ts);
            rotate = match self.rotate_opts.time {
                RotateTime::Year => y != ly,
                RotateTime::Month => m != lm,
                RotateTime::Day => d != ld,
                RotateTime::Second => sec > lsec,
                _ => false,
            }
        }
        if rotate {
            self.flush_buf(&mut state);
            debug_assert!(state.write_buf.is_empty());
            if let Err(e) = self.rotate(&mut state) {
                eprintln!("Warning: log rotate failed");
                eprintln!("{e:#}");
            }
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

    fn rotate(&self, state: &mut FileLoggerState) -> anyhow::Result<()> {
        let main_name = self.file_name.as_str();
        // "a.b.c" => ("a.b", "c")
        // "a" => ("a", "")
        let (stem, ext) = if let Some(dotind) = main_name.rfind('.') {
            (&main_name[..dotind], &main_name[dotind..])
        } else {
            (main_name, "")
        };

        let mut last_no = 0;
        // test ".1" .. ".(file_count - 1)"
        for i in 1..self.rotate_opts.file_count {
            let archive_name = format!("{stem}.{i}{ext}");
            let path = self.dir_path.join(archive_name);
            if path.exists() {
                last_no = i;
            } else {
                break;
            }
        }

        for i in (0..=last_no).rev() {
            let from = if i == 0 {
                self.file_path.clone()
            } else {
                self.dir_path.join(format!("{stem}.{}{ext}", i))
            };
            let to = self.dir_path.join(format!("{stem}.{}{ext}", i + 1));
            std::fs::rename(from, to)?;
        }

        let (mut new_file, size) = open_new_or_append(&self.file_path)?;
        // swap and close
        std::mem::swap(&mut state.file, &mut new_file);
        drop(new_file);
        state.len = size;

        Ok(())
    }
}

/// Open with (create + append), return File and size.
fn open_new_or_append(file_path: impl AsRef<Path>) -> Result<(File, usize), anyhow::Error> {
    let file = File::options().append(true).create(true).open(file_path)?;
    let len = file.metadata()?.len();

    Ok((file, len as usize))
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

fn to_ymd(ts: &DateTime<Local>) -> (i64, u32, u32, u32) {
    (ts.timestamp(), ts.year() as u32, ts.month(), ts.day())
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
        self.buffered_write_entry(&timestamp, &output);
    }

    fn flush(&self) {
        let mut state = self.state.lock().unwrap();
        self.flush_buf(&mut state);
    }
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
