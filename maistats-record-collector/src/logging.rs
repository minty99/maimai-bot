use eyre::WrapErr;
use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, Mutex},
};
use time::{UtcOffset, format_description::well_known::Rfc3339};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, time::OffsetTime, writer::MakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

const DEFAULT_LOG_BUFFER_CAPACITY: usize = 1000;
const JST_UTC_OFFSET_HOURS: i8 = 9;

pub struct LogBuffer {
    capacity: usize,
    entries: Mutex<VecDeque<String>>,
}

pub struct LogSnapshot {
    pub total: usize,
    pub lines: Vec<String>,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            entries: Mutex::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub fn snapshot(&self, limit: usize) -> LogSnapshot {
        let entries = self.entries.lock().expect("log buffer lock poisoned");
        let total = entries.len();
        let skip = total.saturating_sub(limit);

        LogSnapshot {
            total,
            lines: entries.iter().skip(skip).cloned().collect(),
        }
    }

    pub fn make_writer(self: &Arc<Self>) -> LogBufferMakeWriter {
        LogBufferMakeWriter {
            buffer: Arc::clone(self),
        }
    }

    fn push_line(&self, line: String) {
        if line.is_empty() {
            return;
        }

        let mut entries = self.entries.lock().expect("log buffer lock poisoned");
        if entries.len() == self.capacity {
            entries.pop_front();
        }
        entries.push_back(line);
    }
}

#[derive(Clone)]
pub struct LogBufferMakeWriter {
    buffer: Arc<LogBuffer>,
}

impl<'a> MakeWriter<'a> for LogBufferMakeWriter {
    type Writer = LogBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogBufferWriter {
            buffer: Arc::clone(&self.buffer),
            pending: Vec::new(),
        }
    }
}

pub struct LogBufferWriter {
    buffer: Arc<LogBuffer>,
    pending: Vec<u8>,
}

impl LogBufferWriter {
    fn flush_complete_lines(&mut self) {
        while let Some(newline_idx) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut line = self.pending.drain(..=newline_idx).collect::<Vec<_>>();
            if matches!(line.last(), Some(b'\n')) {
                line.pop();
            }
            if matches!(line.last(), Some(b'\r')) {
                line.pop();
            }
            self.buffer
                .push_line(String::from_utf8_lossy(&line).into_owned());
        }
    }
}

impl Write for LogBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        self.flush_complete_lines();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for LogBufferWriter {
    fn drop(&mut self) {
        if self.pending.is_empty() {
            return;
        }

        let line = String::from_utf8_lossy(&self.pending)
            .trim_end_matches('\r')
            .to_string();
        self.buffer.push_line(line);
    }
}

pub fn init_tracing() -> eyre::Result<Arc<LogBuffer>> {
    let log_buffer = Arc::new(LogBuffer::new(DEFAULT_LOG_BUFFER_CAPACITY));
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let timer = OffsetTime::new(
        UtcOffset::from_hms(JST_UTC_OFFSET_HOURS, 0, 0).expect("valid JST UTC offset"),
        Rfc3339,
    );

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_timer(timer.clone()))
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_timer(timer)
                .with_writer(log_buffer.make_writer()),
        )
        .try_init()
        .wrap_err("initialize tracing subscriber")?;

    Ok(log_buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_writer_collects_lines_across_partial_writes() {
        let buffer = Arc::new(LogBuffer::new(8));
        let make_writer = buffer.make_writer();
        let mut writer = make_writer.make_writer();

        writer
            .write_all(b"first line\nsecond")
            .expect("write first chunk");
        writer
            .write_all(b" line\nthird line")
            .expect("write second chunk");
        drop(writer);

        let snapshot = buffer.snapshot(10);
        assert_eq!(snapshot.total, 3);
        assert_eq!(
            snapshot.lines,
            vec![
                "first line".to_string(),
                "second line".to_string(),
                "third line".to_string()
            ]
        );
    }

    #[test]
    fn snapshot_returns_latest_lines_when_limit_is_smaller_than_total() {
        let buffer = LogBuffer::new(3);
        buffer.push_line("one".to_string());
        buffer.push_line("two".to_string());
        buffer.push_line("three".to_string());

        let snapshot = buffer.snapshot(2);
        assert_eq!(snapshot.total, 3);
        assert_eq!(snapshot.lines, vec!["two".to_string(), "three".to_string()]);
    }
}
