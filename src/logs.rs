/*! Logss module for async orchestrator
 * Defines log structures
 */
use std::fmt;
use std::io::Write;

// Log: an append-only fixed size buffer

const BLOCK_SIZE: usize = 64 * 1024;
const TRUNCATION_MSG: &str = "...[ TRUNCATED ]...\n";
const AVAILABLE_SIZE: usize = BLOCK_SIZE - TRUNCATION_MSG.len();

#[derive(Copy, Clone)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARNING,
    ERROR,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DEBUG => write!(f, "DEBUG"),
            Self::INFO => write!(f, "INFO"),
            Self::WARNING => write!(f, "WARNING"),
            Self::ERROR => write!(f, "ERROR"),
        }
    }
}

#[derive(Clone)]
pub struct LogBuffer {
    data: Box<[u8; BLOCK_SIZE]>,
    len: usize,
    full: bool,
}

impl fmt::Display for LogBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let used = &self.data[..self.len];

        match std::str::from_utf8(used) {
            Ok(text) => write!(f, "{text}"),
            Err(_) => write!(f, "<non-utf8 log data>"),
        }
    }
}

impl fmt::Debug for LogBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let used = &self.data[..self.len];

        let text = match std::str::from_utf8(used) {
            Ok(s) => s,
            Err(_) => "<non-utf8 log data>",
        };

        f.debug_struct("LogBuffer")
            .field("len", &self.len)
            .field("full", &self.full)
            .field("data", &text)
            .finish()
    }
}

impl fmt::Write for LogBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl std::io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.full {
            return Ok(0);
        }
        if self.len + buf.len() <= AVAILABLE_SIZE {
            self.write_bytes(buf);
            Ok(buf.len())
        } else {
            self.write_bytes(TRUNCATION_MSG.as_bytes());
            self.full = true;
            Ok(0)
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            len: 0,
            data: Box::new([0; BLOCK_SIZE]),
            full: false,
        }
    }

    pub fn log(&mut self, level: LogLevel, msg: &str) {
        let _ = write!(self, "[{}] {}\n", level, msg);
    }

    pub fn logf(&mut self, level: LogLevel, args: fmt::Arguments<'_>) {
        let _ = self.write_fmt(format_args!("[{}] {}\n", level, args));
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        debug_assert!(self.len + bytes.len() <= BLOCK_SIZE);
        let end = self.len + bytes.len();
        self.data[self.len..end].copy_from_slice(bytes);
        self.advance_len(bytes.len());
    }

    fn advance_len(&mut self, amount: usize) {
        debug_assert!(self.len + amount <= BLOCK_SIZE);
        self.len += amount;
    }
}
