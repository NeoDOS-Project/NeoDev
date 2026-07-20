use crate::config::Config;
use crate::vmm::{HypervisorBackend, NetworkMode, VmConfig, VmInstance, VmStatus};
use anyhow::{Context, Result};
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub struct VirtualBoxBackend;

struct VBoxInstance {
    serial_file: Option<PathBuf>,
    vm_name: String,
}

impl VmInstance for VBoxInstance {
    fn serial_path(&self) -> Option<&Path> { self.serial_file.as_deref() }
    fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<i32>> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match vm_status(&self.vm_name)? { VmStatus::Running | VmStatus::Paused => std::thread::sleep(Duration::from_millis(500)), VmStatus::Stopped => return Ok(Some(0)), VmStatus::NotFound => return Ok(None) }
        }
        Ok(None)
    }
    fn kill(&mut self) -> Result<()> { vm_poweroff(&self.vm_name) }
    fn pid(&self) -> Option<u32> { None }
}

impl HypervisorBackend for VirtualBoxBackend {
    fn name(&self) -> &str { "virtualbox" }

    fn check_prerequisites(&self, _cfg: &Config) -> Result<()> {
        if which("VBoxManage").is_none() { anyhow::bail!("VBoxManage not found. Install VirtualBox and ensure VBoxManage is in PATH."); }
        let output = Command::new("VBoxManage").args(["--version"]).output().context("Failed to run VBoxManage --version")?;
        if output.status.success() { println!("  VirtualBox version: {}", String::from_utf8_lossy(&output.stdout).trim()); }
        else { anyhow::bail!("VBoxManage reported an error. Is VirtualBox properly installed?"); }
        Ok(())
    }

    fn ensure_vm(&self, _cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let name = &vmcfg.name;
        let _ = vm_poweroff(name);
        std::thread::sleep(Duration::from_millis(500));

        if !vmcfg.disk_image.exists() { anyhow::bail!("Disk image not found: {}\nRun 'neodev build --image' first", vmcfg.disk_image.display()); }

        let vdi_path = &vmcfg.disk_vdi;
        if vmcfg.disk_image.exists() { refresh_vdi(vmcfg)?; }

        if vm_exists(name) { println!("  VM '{}' already exists, reconfiguring", name); modify_vm(name, vmcfg)?; attach_storage(name, vdi_path)?; return Ok(()); }

        println!("  Creating VirtualBox VM '{}'...", name);
        let status = Command::new("VBoxManage").args(["createvm", "--name", name, "--ostype", "Linux_64", "--register"]).status().context("Failed to create VM")?;
        if !status.success() { anyhow::bail!("VBoxManage createvm failed"); }
        modify_vm(name, vmcfg)?;
        attach_storage(name, vdi_path)?;
        println!("  VM '{}' created successfully", name);
        Ok(())
    }

    fn delete_vm(&self, _cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let name = &vmcfg.name;
        if !vm_exists(name) { println!("  VM '{}' does not exist", name); return Ok(()); }
        println!("  Deleting VM '{}'...", name);
        Command::new("VBoxManage").args(["unregistervm", name, "--delete"]).status().context("Failed to delete VM")?;
        println!("  VM '{}' deleted", name);
        Ok(())
    }

    fn run(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let name = &vmcfg.name;
        println!("{} NeoDOS VirtualBox Session", "[*]".bold().cyan()); println!();
        self.ensure_vm(cfg, vmcfg)?;

        let start_type = if vmcfg.headless { "headless" } else { "gui" };
        println!("  Starting VM '{}' (type: {})...", name, start_type);
        let output = Command::new("VBoxManage").args(["startvm", name, "--type", start_type])
            .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).output().context("Failed to start VM")?;
        if !output.status.success() { anyhow::bail!("Failed to start VM: {}", String::from_utf8_lossy(&output.stderr)); }

        loop {
            std::thread::sleep(Duration::from_secs(2));
            match vm_status(name)? { VmStatus::Running | VmStatus::Paused => {}, _ => { println!(); println!("{} VM stopped", "[*]".bold().cyan()); break; } }
        }
        Ok(())
    }

    fn start_headless(&self, _cfg: &Config, vmcfg: &VmConfig) -> Result<Box<dyn VmInstance>> {
        let name = &vmcfg.name;
        if !vm_exists(name) { anyhow::bail!("VM '{}' does not exist. Run 'neodev vm create' first.", name); }
        let output = Command::new("VBoxManage").args(["startvm", name, "--type", "headless"]).output().context("Failed to start VM headless")?;
        if !output.status.success() { anyhow::bail!("Failed to start VM headless: {}", String::from_utf8_lossy(&output.stderr)); }
        std::thread::sleep(Duration::from_secs(3));
        Ok(Box::new(VBoxInstance { serial_file: vmcfg.serial_file.clone(), vm_name: name.clone() }))
    }

    fn stop(&self, _cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let name = &vmcfg.name;
        println!("  Stopping VM '{}'...", name);
        let status = Command::new("VBoxManage").args(["controlvm", name, "acpipowerbutton"]).status().context("Failed to send ACPI poweroff")?;
        if status.success() {
            for _ in 0..10 { std::thread::sleep(Duration::from_secs(1)); match vm_status(name)? { VmStatus::Stopped | VmStatus::NotFound => { println!("  VM stopped gracefully"); return Ok(()); } _ => {} } }
            println!("  ACPI timeout, forcing poweroff...");
        }
        let _ = Command::new("VBoxManage").args(["controlvm", name, "poweroff"]).status();
        println!("  VM powered off");
        Ok(())
    }

    fn reset(&self, _cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        Command::new("VBoxManage").args(["controlvm", &vmcfg.name, "reset"]).status().context("Failed to reset VM")?;
        Ok(())
    }

    fn status(&self, _cfg: &Config, vmcfg: &VmConfig) -> Result<VmStatus> { vm_status(&vmcfg.name) }
}

fn which(cmd: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) { let full = dir.join(cmd); if full.is_file() { return Some(full); } }
        None
    })
}

fn vm_exists(name: &str) -> bool {
    Command::new("VBoxManage").args(["showvminfo", name]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

fn vm_status(name: &str) -> Result<VmStatus> {
    if !vm_exists(name) { return Ok(VmStatus::NotFound); }
    let output = Command::new("VBoxManage").args(["showvminfo", name, "--machinereadable"]).output().context("Failed to get VM status")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("VMState=\"running\"") { Ok(VmStatus::Running) }
    else if stdout.contains("VMState=\"paused\"") { Ok(VmStatus::Paused) }
    else { Ok(VmStatus::Stopped) }
}

fn vm_poweroff(name: &str) -> Result<()> {
    let _ = Command::new("VBoxManage").args(["controlvm", name, "poweroff"]).status();
    Ok(())
}

fn convert_to_vdi(raw_path: &Path, vdi_path: &Path) -> Result<()> {
    println!("  Converting {} to VDI...", raw_path.display());
    if vdi_path.exists() { std::fs::remove_file(vdi_path)?; }
    Command::new("VBoxManage").args(["convertfromraw", raw_path.to_str().unwrap(), vdi_path.to_str().unwrap()]).status().context("Failed to convert")?;
    println!("  VDI created: {}", vdi_path.display());
    Ok(())
}

fn refresh_vdi(vmcfg: &VmConfig) -> Result<()> {
    let raw = &vmcfg.disk_image;
    let vdi = &vmcfg.disk_vdi;
    if !vdi.exists() { return convert_to_vdi(raw, vdi); }
    let raw_modified = std::fs::metadata(raw).and_then(|m| m.modified()).ok();
    let vdi_modified = std::fs::metadata(vdi).and_then(|m| m.modified()).ok();
    if match (raw_modified, vdi_modified) { (Some(raw_mtime), Some(vdi_mtime)) => raw_mtime > vdi_mtime, (Some(_), None) => true, _ => false } {
        println!("  Disk image updated, re-converting VDI...");
        convert_to_vdi(raw, vdi)?;
    }
    Ok(())
}

fn modify_vm(name: &str, vmcfg: &VmConfig) -> Result<()> {
    Command::new("VBoxManage").args(["modifyvm", name, "--memory", &vmcfg.memory_mb.to_string()]).status()?;
    Command::new("VBoxManage").args(["modifyvm", name, "--cpus", &vmcfg.cpus.to_string()]).status()?;
    if vmcfg.efi { Command::new("VBoxManage").args(["modifyvm", name, "--firmware", "efi"]).status()?; }
    Command::new("VBoxManage").args(["modifyvm", name, "--chipset", "ich9"]).status()?;

    let serial_path = vmcfg.serial_file.as_deref().map(|p| if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(p) })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("vbox_serial.log"));
    if let Some(parent) = serial_path.parent() { let _ = std::fs::create_dir_all(parent); }
    Command::new("VBoxManage").args(["modifyvm", name, "--uart1", "0x3F8", "4", "--uartmode1", "file", serial_path.to_str().unwrap()]).status()?;

    match vmcfg.network {
        NetworkMode::User => { Command::new("VBoxManage").args(["modifyvm", name, "--nic1", "nat", "--nictype1", "82540EM", "--cableconnected1", "on"]).status()?; }
        NetworkMode::Bridged => {
            let bridge_iface = detect_bridge_interface();
            Command::new("VBoxManage").args(["modifyvm", name, "--nic1", "bridged", "--bridgeadapter1", &bridge_iface, "--nictype1", "82540EM", "--cableconnected1", "on", "--nicpromisc1", "allow-all"]).status()?;
            println!("  Bridged network via: {}", bridge_iface);
        }
        NetworkMode::None => { Command::new("VBoxManage").args(["modifyvm", name, "--nic1", "none"]).status()?; }
    }
    Ok(())
}

fn attach_storage(name: &str, vdi_path: &Path) -> Result<()> {
    let _ = Command::new("VBoxManage").args(["storageattach", name, "--storagectl", "AHCI", "--port", "0", "--device", "0", "--type", "hdd", "--medium", "none"]).status();
    let _ = Command::new("VBoxManage").args(["closemedium", "disk", vdi_path.to_str().unwrap()]).status();
    let _ = Command::new("VBoxManage").args(["storagectl", name, "--name", "AHCI", "--remove"]).status();
    Command::new("VBoxManage").args(["storagectl", name, "--name", "AHCI", "--add", "sata", "--controller", "IntelAhci"]).status()?;
    Command::new("VBoxManage").args(["storageattach", name, "--storagectl", "AHCI", "--port", "0", "--device", "0", "--type", "hdd", "--medium", vdi_path.to_str().unwrap()]).status()?;
    Ok(())
}

fn detect_bridge_interface() -> String {
    let mut ethernet_candidates: Vec<String> = Vec::new();
    let mut wifi_candidates: Vec<String> = Vec::new();
    let mut fallback: Option<String> = None;

    if let Ok(output) = Command::new("ip").args(["-o", "link", "show"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 { continue; }
            let iface = parts[1].trim().to_string();
            if iface == "lo" || iface.starts_with("tap") || iface.starts_with("docker") || iface.starts_with("vbox") || iface.starts_with("virbr") || iface.starts_with("br-") { continue; }
            let is_up = std::fs::read_to_string(format!("/sys/class/net/{}/operstate", iface)).map(|s| s.trim() == "up").unwrap_or(false);
            if !is_up { continue; }
            let has_carrier = std::fs::read_to_string(format!("/sys/class/net/{}/carrier", iface)).map(|s| s.trim() == "1").unwrap_or(false);
            if !has_carrier { continue; }
            let is_wireless = std::fs::read_to_string(format!("/sys/class/net/{}/uevent", iface)).map(|s| s.contains("DEVTYPE=wlan") || s.contains("DEVTYPE=wifi")).unwrap_or(false);
            let is_ethernet = !is_wireless;
            let has_ip = has_ip_address(&iface);
            if is_ethernet { if has_ip { ethernet_candidates.push(iface.clone()); } if fallback.is_none() { fallback = Some(iface.clone()); } }
            else { wifi_candidates.push(iface.clone()); }
        }
    }

    let selected = ethernet_candidates.first().or_else(|| wifi_candidates.first()).or_else(|| fallback.as_ref()).map(|s| s.to_string()).unwrap_or_else(|| "eth0".to_string());
    println!("  Selected: {} for bridged networking", selected);
    selected
}

fn has_ip_address(iface: &str) -> bool {
    Command::new("ip").args(["-4", "addr", "show", "dev", iface]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains("inet ")).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bridge_interface_not_empty() {
        let iface = detect_bridge_interface();
        assert!(!iface.is_empty(), "interface name should not be empty");
        assert!(!iface.contains(' '), "interface name should not contain spaces");
        assert_ne!(iface, "lo", "should not return loopback");
    }

    #[test]
    fn test_detect_returns_real_ethernet() {
        let output = Command::new("ip").args(["-o", "link", "show"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let active_eth: Vec<String> = stdout.lines().filter_map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 { return None; }
            let iface = parts[1].trim().to_string();
            if iface == "lo" { return None; }
            let up = std::fs::read_to_string(format!("/sys/class/net/{}/operstate", iface)).map(|s| s.trim() == "up").unwrap_or(false);
            let car = std::fs::read_to_string(format!("/sys/class/net/{}/carrier", iface)).map(|s| s.trim() == "1").unwrap_or(false);
            if up && car { Some(iface) } else { None }
        }).collect();

        if !active_eth.is_empty() {
            let selected = detect_bridge_interface();
            assert!(!selected.is_empty());
            assert_ne!(selected, "eth0", "should detect real interface, not fallback");
        }
    }

    #[test]
    fn test_has_ip_works() {
        let output = Command::new("ip").args(["-o", "link", "show"]).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 { continue; }
            let iface = parts[1].trim().to_string();
            if iface == "lo" { continue; }
            let has_ip = has_ip_address(&iface);
            let check = Command::new("ip").args(["-4", "addr", "show", "dev", &iface]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains("inet ")).unwrap_or(false);
            assert_eq!(has_ip, check, "has_ip mismatch for {}", iface);
        }
    }
}
