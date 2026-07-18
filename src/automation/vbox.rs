use anyhow::{Context, Result};
use std::process::Command;

pub struct VirtualBoxAutomation {
    vm_name: String,
}

impl VirtualBoxAutomation {
    pub fn new(vm_name: &str) -> Self {
        Self {
            vm_name: vm_name.to_string(),
        }
    }
}

impl super::VmAutomation for VirtualBoxAutomation {
    fn send_keys(&self, keys: &str) -> Result<()> {
        let status = Command::new("VBoxManage")
            .args([
                "controlvm",
                &self.vm_name,
                "keyboardputstring",
                keys,
            ])
            .status()
            .context("Failed to execute VBoxManage keyboardputstring")?;

        if !status.success() {
            anyhow::bail!(
                "VBoxManage keyboardputstring failed for VM '{}'",
                self.vm_name
            );
        }
        Ok(())
    }

    fn send_enter(&self) -> Result<()> {
        // PS/2 scancodes: make=0x1C, break=0x9C
        let status = Command::new("VBoxManage")
            .args([
                "controlvm",
                &self.vm_name,
                "keyboardputscancode",
                "1c",
                "9c",
            ])
            .status()
            .context("Failed to execute VBoxManage keyboardputscancode")?;

        if !status.success() {
            anyhow::bail!(
                "VBoxManage keyboardputscancode (Enter) failed for VM '{}'",
                self.vm_name
            );
        }
        Ok(())
    }

    fn send_command(&self, cmd: &str) -> Result<()> {
        // VirtualBox can inject the full string + Enter in one go by appending \n
        let text = format!("{}\n", cmd);
        let status = Command::new("VBoxManage")
            .args([
                "controlvm",
                &self.vm_name,
                "keyboardputstring",
                &text,
            ])
            .status()
            .context("Failed to execute VBoxManage keyboardputstring")?;

        if !status.success() {
            anyhow::bail!(
                "VBoxManage keyboardputstring failed for VM '{}'",
                self.vm_name
            );
        }
        Ok(())
    }
}
