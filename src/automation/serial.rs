use anyhow::Result;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct SerialMonitor {
    path: PathBuf,
    last_offset: u64,
}

impl SerialMonitor {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            last_offset: 0,
        }
    }

    pub fn read_new(&mut self) -> Result<String> {
        if !self.path.exists() {
            return Ok(String::new());
        }
        let file = std::fs::File::open(&self.path)?;
        let metadata = file.metadata()?;
        let file_len = metadata.len();
        if file_len <= self.last_offset {
            return Ok(String::new());
        }
        let read_size = (file_len - self.last_offset) as usize;
        let mut buf = vec![0u8; read_size];
        file.read_exact_at(&mut buf, self.last_offset)?;
        self.last_offset = file_len;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    pub fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<bool> {
        let deadline = Instant::now() + timeout;
        let mut accumulated = String::new();
        while Instant::now() < deadline {
            if let Ok(data) = self.read_new() {
                accumulated.push_str(&data);
                if accumulated.contains(pattern) {
                    return Ok(true);
                }
                if accumulated.len() > 8192 {
                    accumulated = accumulated.split_off(accumulated.len() - 4096);
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(accumulated.contains(pattern))
    }

    pub fn wait_for_boot_complete(&mut self, timeout: Duration) -> Result<bool> {
        let markers = &["Type HELP", "NeoShell", "starting shell"];
        let deadline = Instant::now() + timeout;
        let mut buf = String::new();
        while Instant::now() < deadline {
            if let Ok(data) = self.read_new() {
                buf.push_str(&data);
                for marker in markers {
                    if buf.contains(marker) {
                        return Ok(true);
                    }
                }
                if buf.len() > 16384 {
                    buf = buf.split_off(buf.len() - 8192);
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(false)
    }
}
