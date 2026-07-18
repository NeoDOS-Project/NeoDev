mod qemu;
mod serial;
mod vbox;

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub trait VmAutomation: Send {
    fn send_keys(&self, keys: &str) -> Result<()>;
    fn send_enter(&self) -> Result<()>;
    fn send_command(&self, cmd: &str) -> Result<()> {
        self.send_keys(cmd)?;
        self.send_enter()?;
        Ok(())
    }
}

#[allow(dead_code)]
pub struct AutomationConfig {
    pub backend: String,
    pub vm_name: String,
    pub monitor_port: u16,
    pub serial_path: Option<PathBuf>,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            backend: "qemu".into(),
            vm_name: "NeoDOS".into(),
            monitor_port: 4445,
            serial_path: None,
        }
    }
}

pub fn create_automation(cfg: &AutomationConfig) -> Result<Box<dyn VmAutomation>> {
    match cfg.backend.as_str() {
        "qemu" => Ok(Box::new(qemu::QemuAutomation::new(cfg.monitor_port)?)),
        "virtualbox" => Ok(Box::new(vbox::VirtualBoxAutomation::new(&cfg.vm_name))),
        other => anyhow::bail!("Automation not supported for backend '{}'", other),
    }
}

pub fn create_serial_monitor(path: &Path) -> serial::SerialMonitor {
    serial::SerialMonitor::new(path)
}

pub fn wait_for_prompt(
    monitor: &mut serial::SerialMonitor,
    timeout: Duration,
) -> Result<bool> {
    // The NeoDOS shell prompt ends with "> "
    let prompt_pattern = "> ";
    let deadline = std::time::Instant::now() + timeout;
    let mut buf = String::new();

    while std::time::Instant::now() < deadline {
        if let Ok(data) = monitor.read_new() {
            buf.push_str(&data);
            if buf.contains(prompt_pattern) {
                return Ok(true);
            }
            // Keep only the last 4KB to avoid unbounded memory
            if buf.len() > 4096 {
                buf = buf.split_off(buf.len() - 2048);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    Ok(buf.contains(prompt_pattern))
}
