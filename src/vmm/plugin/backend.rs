use super::abi::{
    NeodevBackendV1, PluginConfig, PluginVmConfig, PluginVmInstanceV1, PluginVmStatus,
};
use super::loader::LoadedPlugin;
use crate::config::Config;
use crate::vmm::{HypervisorBackend, NetworkMode, StorageMode, VmConfig, VmInstance, VmStatus};
use anyhow::Result;
use std::ffi::CString;
use std::path::Path;
use std::time::Duration;

pub struct PluginBackend {
    pub plugin: &'static NeodevBackendV1,
    ctx: *mut std::ffi::c_void,
    pub name: String,
}

unsafe impl Send for PluginBackend {}
unsafe impl Sync for PluginBackend {}

impl PluginBackend {
    pub fn new(loaded: &LoadedPlugin) -> Self {
        let ctx = (loaded.backend.create)();
        let name = unsafe { std::ffi::CStr::from_ptr(loaded.backend.name) }
            .to_str()
            .unwrap_or("unknown")
            .to_owned();
        PluginBackend {
            plugin: loaded.backend,
            ctx,
            name,
        }
    }
}

impl Drop for PluginBackend {
    fn drop(&mut self) {
        (self.plugin.destroy)(self.ctx);
    }
}

fn to_plugin_config(cfg: &Config) -> Result<(PluginConfig, Vec<CString>)> {
    let roots = CString::new(cfg.neodos_root.to_str().unwrap_or(""))?;
    let net = CString::new(cfg.vm_network.as_str())?;
    let ovmf_c = CString::new(cfg.ovmf_code.to_str().unwrap_or(""))?;
    let ovmf_v = CString::new(cfg.ovmf_vars_template.to_str().unwrap_or(""))?;
    let kt = CString::new(cfg.kernel_target.as_str())?;
    let bt = CString::new(cfg.bootloader_target.as_str())?;

    let plugin_cfg = PluginConfig {
        neodos_root: roots.as_ptr(),
        esp_size_mb: cfg.esp_size_mb,
        neodos_size_mb: cfg.neodos_size_mb,
        vm_memory_mb: cfg.vm_memory_mb,
        vm_cpus: cfg.vm_cpus,
        vm_network: net.as_ptr(),
        qemu_kvm: cfg.qemu_kvm,
        qemu_bdm: cfg.qemu_bdm,
        ovmf_code: ovmf_c.as_ptr(),
        ovmf_vars_template: ovmf_v.as_ptr(),
        kernel_target: kt.as_ptr(),
        bootloader_target: bt.as_ptr(),
        debug: cfg.debug,
    };
    Ok((plugin_cfg, vec![roots, net, ovmf_c, ovmf_v, kt, bt]))
}

fn to_plugin_vmcfg(vmcfg: &VmConfig) -> Result<(PluginVmConfig, Vec<CString>)> {
    let name = CString::new(vmcfg.name.as_str())?;
    let di = CString::new(vmcfg.disk_image.to_str().unwrap_or(""))?;
    let dv = CString::new(vmcfg.disk_vdi.to_str().unwrap_or(""))?;
    let sf = vmcfg.serial_file.as_ref().and_then(|p| {
        CString::new(p.to_str().unwrap_or("")).ok()
    });

    let network_val = match vmcfg.network {
        NetworkMode::User => 0i32,
        NetworkMode::Bridged => 1i32,
        NetworkMode::None => 2i32,
    };

    let storage_val = match vmcfg.storage_mode {
        StorageMode::Ahci => 0i32,
        StorageMode::Ata => 1i32,
        StorageMode::Nvme => 2i32,
        StorageMode::Virtio => 3i32,
    };

    let mut cstrings = vec![name.clone(), di.clone(), dv.clone()];
    let serial_ptr = if let Some(ref sf) = sf {
        cstrings.push(sf.clone());
        sf.as_ptr()
    } else {
        std::ptr::null()
    };

    let plugin_vmcfg = PluginVmConfig {
        name: name.as_ptr(),
        memory_mb: vmcfg.memory_mb,
        cpus: vmcfg.cpus,
        efi: vmcfg.efi,
        disk_image: di.as_ptr(),
        disk_vdi: dv.as_ptr(),
        serial_file: serial_ptr,
        network: network_val,
        headless: vmcfg.headless,
        gdb: vmcfg.gdb,
        gdb_port: vmcfg.gdb_port,
        storage_mode: storage_val,
    };
    Ok((plugin_vmcfg, cstrings))
}

impl HypervisorBackend for PluginBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn check_prerequisites(&self, cfg: &Config) -> Result<()> {
        let (pcfg, _keep) = to_plugin_config(cfg)?;
        let rc = (self.plugin.check_prerequisites)(self.ctx, &pcfg);
        if rc != 0 {
            anyhow::bail!("Plugin '{}' check_prerequisites failed (code {})", self.name, rc);
        }
        Ok(())
    }

    fn ensure_vm(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let rc = (self.plugin.ensure_vm)(self.ctx, &pcfg, &pvmcfg);
        if rc != 0 {
            anyhow::bail!("Plugin '{}' ensure_vm failed (code {})", self.name, rc);
        }
        Ok(())
    }

    fn delete_vm(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let rc = (self.plugin.delete_vm)(self.ctx, &pcfg, &pvmcfg);
        if rc != 0 {
            anyhow::bail!("Plugin '{}' delete_vm failed (code {})", self.name, rc);
        }
        Ok(())
    }

    fn run(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let rc = (self.plugin.run)(self.ctx, &pcfg, &pvmcfg);
        if rc != 0 {
            anyhow::bail!("Plugin '{}' run failed (code {})", self.name, rc);
        }
        Ok(())
    }

    fn start_headless(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<Box<dyn VmInstance>> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let instance_ptr = (self.plugin.start_headless)(self.ctx, &pcfg, &pvmcfg);
        if instance_ptr.is_null() {
            anyhow::bail!("Plugin '{}' start_headless returned null", self.name);
        }
        Ok(Box::new(PluginVmInstance {
            vtable: instance_ptr as *const PluginVmInstanceV1,
            ctx: instance_ptr,
        }))
    }

    fn stop(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let rc = (self.plugin.stop)(self.ctx, &pcfg, &pvmcfg);
        if rc != 0 {
            anyhow::bail!("Plugin '{}' stop failed (code {})", self.name, rc);
        }
        Ok(())
    }

    fn reset(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<()> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let rc = (self.plugin.reset)(self.ctx, &pcfg, &pvmcfg);
        if rc != 0 {
            anyhow::bail!("Plugin '{}' reset failed (code {})", self.name, rc);
        }
        Ok(())
    }

    fn status(&self, cfg: &Config, vmcfg: &VmConfig) -> Result<VmStatus> {
        let (pcfg, _k1) = to_plugin_config(cfg)?;
        let (pvmcfg, _k2) = to_plugin_vmcfg(vmcfg)?;
        let s = (self.plugin.status)(self.ctx, &pcfg, &pvmcfg);
        Ok(match s {
            PluginVmStatus::Running => VmStatus::Running,
            PluginVmStatus::Paused => VmStatus::Paused,
            PluginVmStatus::Stopped => VmStatus::Stopped,
            PluginVmStatus::NotFound => VmStatus::NotFound,
        })
    }
}

struct PluginVmInstance {
    vtable: *const PluginVmInstanceV1,
    ctx: *mut std::ffi::c_void,
}

unsafe impl Send for PluginVmInstance {}

impl VmInstance for PluginVmInstance {
    fn serial_path(&self) -> Option<&Path> {
        let vtable = unsafe { &*self.vtable };
        let path_fn = vtable.serial_path?;
        let ptr = path_fn(self.ctx);
        if ptr.is_null() {
            return None;
        }
        let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_str()
            .ok()?;
        Some(Path::new(s))
    }

    fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<i32>> {
        let vtable = unsafe { &*self.vtable };
        let wait_fn = vtable.wait_timeout.ok_or_else(|| {
            anyhow::anyhow!("Plugin instance does not support wait_timeout")
        })?;
        let rc = wait_fn(self.ctx, timeout.as_millis() as u64);
        Ok(if rc < 0 { None } else { Some(rc) })
    }

    fn kill(&mut self) -> Result<()> {
        let vtable = unsafe { &*self.vtable };
        let kill_fn = vtable.kill.ok_or_else(|| {
            anyhow::anyhow!("Plugin instance does not support kill")
        })?;
        let rc = kill_fn(self.ctx);
        if rc != 0 {
            anyhow::bail!("Plugin instance kill returned {}", rc);
        }
        Ok(())
    }

    fn pid(&self) -> Option<u32> {
        let vtable = unsafe { &*self.vtable };
        let pid_fn = vtable.pid?;
        Some(pid_fn(self.ctx))
    }
}

impl Drop for PluginVmInstance {
    fn drop(&mut self) {
        if let Some(destroy) = unsafe { &*self.vtable }.destroy {
            destroy(self.ctx);
        }
    }
}
