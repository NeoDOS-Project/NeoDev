use crate::config::Config;
use crate::vmm::{HypervisorBackend, NetworkMode, VmConfig, VmInstance, VmStatus};
use anyhow::{Context, Result};
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

pub struct VirtualBoxBackend;

struct VBoxInstance {
    #[allow(dead_code)]
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

        // Single authoritative raw image -> VDI synchronization path. Handles a
        // missing raw image (error), a missing VDI (convert) and a stale VDI
        // (reconvert), detaching the existing medium first when the VM exists.
        ensure_vdi_current(vmcfg)?;

        let vdi_path = &vmcfg.disk_vdi;

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
        if matches!(vm_status(name)?, VmStatus::Running | VmStatus::Paused) {
            anyhow::bail!("VM '{}' is already running. Stop it before running tests.", name);
        }
        // `neodev test` reaches the VM through this entry point, so the VDI must
        // be refreshed here too rather than only in `ensure_vm`.
        ensure_vdi_current(vmcfg)?;
        // Point the guest COM1 at this session's serial log so the harness can
        // read the boot/tests. Without this, headless tests write to whatever
        // log the previous `run` configured and the reader sees nothing.
        configure_serial(name, vmcfg)?;
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

/// Result of the raw image -> VDI synchronization decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdiSyncOutcome {
    /// The existing VDI is already at least as new as the raw image.
    Current,
    /// The VDI was created or regenerated from the raw image.
    Converted,
}

/// Deterministic freshness policy for the raw image -> VDI relation.
///
/// A conversion is required when the VDI is missing (`vdi_mtime == None`) or
/// when the raw image is strictly newer than the VDI. Equal timestamps mean the
/// VDI is current (no fixed fudge factor / sleep is used).
pub fn needs_vdi_conversion(img_mtime: SystemTime, vdi_mtime: Option<SystemTime>) -> bool {
    match vdi_mtime {
        None => true,
        Some(vdi_mtime) => img_mtime > vdi_mtime,
    }
}

/// Read the modification time of the raw image, failing clearly if it is absent
/// (Case D: the caller must not silently continue with a stale VDI).
fn raw_image_mtime(raw: &Path) -> Result<SystemTime> {
    let meta = std::fs::metadata(raw).with_context(|| {
        format!(
            "Raw disk image not found: {}\nRun 'neodev build --image' first.",
            raw.display()
        )
    })?;
    meta.modified()
        .with_context(|| format!("Cannot read modification time of {}", raw.display()))
}

/// Modification time of a path, or `None` when it does not exist / is unreadable.
fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Single authoritative raw image -> VDI synchronization path.
///
/// Owns the freshness policy and the attachment lifecycle so that `run`
/// (`ensure_vm`) and `test` (`start_headless`) cannot drift apart. It:
///   1. fails clearly when the raw image is missing;
///   2. converts when the VDI is missing or older than the raw image;
///   3. refuses to touch the medium while the VM is running;
///   4. detaches the medium before replacing the VDI and re-attaches it after;
///   5. verifies the resulting VDI exists and is at least as new as the raw image.
pub fn ensure_vdi_current(vmcfg: &VmConfig) -> Result<VdiSyncOutcome> {
    let raw = &vmcfg.disk_image;
    let vdi = &vmcfg.disk_vdi;

    // Case D: the raw image must exist. Never fall back to a stale VDI.
    let img_mtime = raw_image_mtime(raw)?;

    let vdi_mtime = file_mtime(vdi);
    if !needs_vdi_conversion(img_mtime, vdi_mtime) {
        // Case B: VDI is at least as new as the raw image.
        return Ok(VdiSyncOutcome::Current);
    }

    let name = &vmcfg.name;
    let vm_present = vm_exists(name);
    if vm_present {
        let status = vm_status(name)?;
        if matches!(status, VmStatus::Running | VmStatus::Paused) {
            anyhow::bail!(
                "VM '{}' is {} and its disk image is stale (raw image '{}' is newer than VDI '{}').\n\
                 Stop the VM before running so NeoDev can regenerate the VDI.",
                name,
                if status == VmStatus::Running { "running" } else { "paused" },
                raw.display(),
                vdi.display()
            );
        }
        // Case A/C with an existing VM: detach before touching the file so we
        // never delete or overwrite a VDI that is currently attached.
        detach_medium(name, vdi)?;
    }

    if vdi_mtime.is_some() {
        println!("  Disk image is newer than VDI, re-converting...");
    } else {
        println!("  VDI '{}' not found, creating...", vdi.display());
    }

    // Case A/C/E: convert and fail loudly on any error.
    convert_to_vdi(raw, vdi)?;

    // Post-conditions: the VDI must exist and be at least as new as the raw image.
    let new_vdi_mtime = file_mtime(vdi).ok_or_else(|| {
        anyhow::anyhow!(
            "VDI '{}' does not exist after converting from '{}'",
            vdi.display(),
            raw.display()
        )
    })?;
    if new_vdi_mtime < img_mtime {
        anyhow::bail!(
            "VDI '{}' is still older than raw image '{}' after conversion (vdi={:?}, img={:?})",
            vdi.display(),
            raw.display(),
            new_vdi_mtime,
            img_mtime
        );
    }

    if vm_present {
        attach_medium(name, vdi)?;
    }

    println!("  VDI is current: {}", vdi.display());
    Ok(VdiSyncOutcome::Converted)
}

fn convert_to_vdi(raw_path: &Path, vdi_path: &Path) -> Result<()> {
    convert_to_vdi_with(Path::new("VBoxManage"), raw_path, vdi_path)
}

/// Run `VBoxManage convertfromraw`, returning a rich error (source image,
/// destination VDI, exit status and captured output) on failure. Takes the
/// VBoxManage path so the conversion can be exercised by tests without a
/// VirtualBox installation.
fn convert_to_vdi_with(vboxmanage: &Path, raw_path: &Path, vdi_path: &Path) -> Result<()> {
    println!("  Converting {} -> {}...", raw_path.display(), vdi_path.display());
    if vdi_path.exists() {
        std::fs::remove_file(vdi_path).with_context(|| {
            format!("Failed to remove stale VDI {}", vdi_path.display())
        })?;
    }
    let output = Command::new(vboxmanage)
        .arg("convertfromraw")
        .arg(raw_path)
        .arg(vdi_path)
        .arg("--format")
        .arg("VDI")
        .output()
        .with_context(|| {
            format!(
                "Failed to run VBoxManage convertfromraw ({} -> {})",
                raw_path.display(),
                vdi_path.display()
            )
        })?;
    if !output.status.success() {
        anyhow::bail!(
            "VBoxManage convertfromraw failed\n  source image: {}\n  destination VDI: {}\n  exit status: {}\n  stdout: {}\n  stderr: {}",
            raw_path.display(),
            vdi_path.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim(),
        );
    }
    if !vdi_path.exists() {
        anyhow::bail!(
            "VBoxManage convertfromraw reported success but '{}' was not created",
            vdi_path.display()
        );
    }
    println!("  VDI created: {}", vdi_path.display());
    Ok(())
}

/// Find the storage slot (`port`, `device`) that currently holds `vdi`, if any.
fn find_attached_slot(name: &str, vdi: &Path) -> Option<(String, String)> {
    let output = Command::new("VBoxManage")
        .args(["showvminfo", name, "--machinereadable"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let info = String::from_utf8_lossy(&output.stdout);
    let vdi_str = vdi.to_string_lossy();
    for line in info.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once('=') else { continue; };
        if value.trim_matches('"') != vdi_str {
            continue;
        }
        // Attached media are reported as e.g. "AHCI-0-0"="/path/disk_image.vdi".
        let key = key.trim_matches('"');
        let mut parts = key.rsplitn(3, '-');
        let device = parts.next()?;
        let port = parts.next()?;
        if !port.is_empty() && !device.is_empty() {
            return Some((port.to_string(), device.to_string()));
        }
    }
    None
}

/// Detach `vdi` from the VM (if attached) and unregister the medium record,
/// without deleting the file. Used before a regeneration or (re)attachment.
fn detach_medium(name: &str, vdi: &Path) -> Result<()> {
    if let Some((port, device)) = find_attached_slot(name, vdi) {
        let status = Command::new("VBoxManage")
            .args([
                "storageattach", name, "--storagectl", "AHCI",
                "--port", &port, "--device", &device,
                "--type", "hdd", "--medium", "none",
            ])
            .status()
            .context("Failed to detach VDI from VM")?;
        if !status.success() {
            anyhow::bail!("Failed to detach '{}' from VM '{}'", vdi.display(), name);
        }
        println!("  Detached {} from VM '{}'", vdi.display(), name);
    }
    // Drop the stale medium registration. The file is left in place; the
    // conversion step is responsible for replacing it.
    let _ = Command::new("VBoxManage").arg("closemedium").arg("disk").arg(vdi).status();
    Ok(())
}

fn ahci_controller_exists(name: &str) -> bool {
    Command::new("VBoxManage")
        .args(["showvminfo", name, "--machinereadable"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout).lines().any(|l| {
                let l = l.trim();
                l.starts_with("storagecontrollername") && l.ends_with("\"AHCI\"")
            })
        })
        .unwrap_or(false)
}

/// Ensure the AHCI controller exists, creating it only when missing so an
/// existing controller (and its other settings) is preserved.
fn ensure_ahci_controller(name: &str) -> Result<()> {
    if ahci_controller_exists(name) {
        return Ok(());
    }
    let status = Command::new("VBoxManage")
        .args(["storagectl", name, "--name", "AHCI", "--add", "sata", "--controller", "IntelAhci"])
        .status()
        .context("Failed to create AHCI controller")?;
    if !status.success() {
        anyhow::bail!("Failed to create AHCI controller on VM '{}'", name);
    }
    Ok(())
}

/// Attach `vdi` to the VM's AHCI port 0 / device 0, creating the controller if
/// needed. Other VM settings (firmware, chipset, network, MAC) are untouched.
fn attach_medium(name: &str, vdi: &Path) -> Result<()> {
    ensure_ahci_controller(name)?;
    let status = Command::new("VBoxManage")
        .args([
            "storageattach", name, "--storagectl", "AHCI",
            "--port", "0", "--device", "0",
            "--type", "hdd",
        ])
        .arg("--medium")
        .arg(vdi)
        .status()
        .context("Failed to attach VDI to VM")?;
    if !status.success() {
        anyhow::bail!("Failed to attach '{}' to VM '{}'", vdi.display(), name);
    }
    Ok(())
}

fn modify_vm(name: &str, vmcfg: &VmConfig) -> Result<()> {
    Command::new("VBoxManage").args(["modifyvm", name, "--memory", &vmcfg.memory_mb.to_string()]).status()?;
    Command::new("VBoxManage").args(["modifyvm", name, "--cpus", &vmcfg.cpus.to_string()]).status()?;
    if vmcfg.efi { Command::new("VBoxManage").args(["modifyvm", name, "--firmware", "efi"]).status()?; }
    Command::new("VBoxManage").args(["modifyvm", name, "--chipset", "ich9"]).status()?;

    configure_serial(name, vmcfg)?;

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

/// Point the guest COM1 at `vmcfg.serial_file` (or `vbox_serial.log` in the
/// current directory when unset). Shared by `modify_vm` (interactive runs) and
/// `start_headless` (test/dhcp), so both produce a readable serial log.
fn configure_serial(name: &str, vmcfg: &VmConfig) -> Result<()> {
    let serial_path = vmcfg.serial_file.as_deref()
        .map(|p| if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().unwrap_or_default().join(p) })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("vbox_serial.log"));
    if let Some(parent) = serial_path.parent() { let _ = std::fs::create_dir_all(parent); }
    let status = Command::new("VBoxManage")
        .args(["modifyvm", name, "--uart1", "0x3F8", "4", "--uartmode1", "file"])
        .arg(&serial_path)
        .status()
        .context("Failed to configure VM serial port")?;
    if !status.success() {
        anyhow::bail!("Failed to configure serial port on VM '{}'", name);
    }
    Ok(())
}

fn attach_storage(name: &str, vdi_path: &Path) -> Result<()> {
    // Detach any medium at the NeoDev slot and unregister the old record, then
    // re-attach the (possibly regenerated) VDI. The controller is preserved.
    detach_medium(name, vdi_path)?;
    attach_medium(name, vdi_path)?;
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
            if is_ethernet {
                if has_ip { ethernet_candidates.push(iface.clone()); }
                if fallback.is_none() { fallback = Some(iface.clone()); }
            } else {
                wifi_candidates.push(iface.clone());
            }
        }
    }

    let selected = ethernet_candidates.first().or_else(|| wifi_candidates.first()).or(fallback.as_ref()).map(|s| s.to_string()).unwrap_or_else(|| "eth0".to_string());
    println!("  Selected: {} for bridged networking", selected);
    selected
}

fn has_ip_address(iface: &str) -> bool {
    Command::new("ip").args(["-4", "addr", "show", "dev", iface]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains("inet ")).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

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

    // ---- VDI freshness policy (deterministic, no VirtualBox required) ----

    fn t(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn missing_vdi_requires_conversion() {
        assert!(needs_vdi_conversion(t(10), None));
    }

    #[test]
    fn newer_vdi_does_not_require_conversion() {
        assert!(!needs_vdi_conversion(t(10), Some(t(20))));
    }

    #[test]
    fn equal_timestamps_do_not_require_conversion() {
        assert!(!needs_vdi_conversion(t(10), Some(t(10))));
    }

    #[test]
    fn newer_img_requires_conversion() {
        assert!(needs_vdi_conversion(t(30), Some(t(20))));
    }

    #[test]
    fn missing_raw_image_is_a_clear_error() {
        let dir = unique_temp_dir("missing-img");
        let err = raw_image_mtime(&dir.join("does-not-exist.img")).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("not found"), "unexpected error: {msg}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conversion_reports_missing_raw_image_through_ensure_path() {
        // ensure_vdi_current must fail on a missing IMG before ever invoking
        // VBoxManage/Raw VDI logic (Case D).
        let dir = unique_temp_dir("ensure-missing-img");
        let vmcfg = VmConfig {
            name: "NeoDOS-never-exists".into(),
            memory_mb: 512,
            cpus: 1,
            efi: true,
            disk_image: dir.join("missing.img"),
            disk_vdi: dir.join("missing.vdi"),
            serial_file: None,
            network: NetworkMode::None,
            headless: true,
            gdb: false,
            gdb_port: 1234,
            storage_mode: crate::vmm::StorageMode::Ahci,
        };
        let err = ensure_vdi_current(&vmcfg).unwrap_err();
        assert!(format!("{:#}", err).contains("not found"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn unique_temp_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "neodev-vbox-test-{}-{}-{}",
            std::process::id(),
            tag,
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, body).unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn conversion_failure_propagates_source_dest_and_output() {
        let dir = unique_temp_dir("conv-fail");
        let raw = dir.join("disk_image.img");
        let vdi = dir.join("disk_image.vdi");
        std::fs::write(&raw, b"raw").unwrap();
        let script = dir.join("fake-VBoxManage");
        write_executable(&script, "#!/bin/sh\necho 'boom failure' >&2\nexit 3\n");

        let err = convert_to_vdi_with(&script, &raw, &vdi).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("disk_image.img"), "source image missing from error: {msg}");
        assert!(msg.contains("disk_image.vdi"), "destination VDI missing from error: {msg}");
        assert!(msg.contains("boom failure"), "VBoxManage output missing from error: {msg}");
        assert!(msg.contains('3'), "exit status missing from error: {msg}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn conversion_preserves_paths_with_spaces() {
        let dir = unique_temp_dir("spaces");
        let spaced = dir.join("neo dev dir");
        std::fs::create_dir_all(&spaced).unwrap();
        let raw = spaced.join("disk image.img");
        let vdi = spaced.join("disk image.vdi");
        std::fs::write(&raw, b"raw").unwrap();

        let script = dir.join("fake-VBoxManage");
        let log = dir.join("args.log");
        let body = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n: > \"$3\"\nexit 0\n",
            log.display()
        );
        write_executable(&script, &body);

        convert_to_vdi_with(&script, &raw, &vdi).unwrap();
        let logged = std::fs::read_to_string(&log).unwrap();
        assert!(logged.contains(&*raw.to_string_lossy()), "raw path lost: {logged}");
        assert!(logged.contains(&*vdi.to_string_lossy()), "vdi path lost: {logged}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "requires a VirtualBox installation with VBoxManage in PATH"]
    fn converts_raw_image_with_real_vboxmanage() {
        let dir = unique_temp_dir("real-vbox");
        let raw = dir.join("disk_image.img");
        // convertfromraw requires a size that is a multiple of 512 bytes.
        std::fs::write(&raw, vec![0u8; 1024 * 1024]).unwrap();
        let vdi = dir.join("disk_image.vdi");
        convert_to_vdi(&raw, &vdi).unwrap();
        assert!(vdi.exists(), "VDI was not created by VBoxManage");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
