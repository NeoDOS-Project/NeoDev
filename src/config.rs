use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub neodos_root: PathBuf,
    pub esp_size_mb: u64,
    pub neodos_size_mb: u64,
    pub gpt_padding_mb: u64,
    pub ovmf_code: PathBuf,
    pub ovmf_vars_template: PathBuf,
    pub vm_backend: String,
    pub vm_memory_mb: u32,
    pub vm_cpus: u32,
    pub vm_network: String,
    pub qemu_kvm: bool,
    pub qemu_bdm: bool,
    pub kernel_target: String,
    pub bootloader_target: String,
    pub debug: bool,
}

impl Config {
    pub fn load(neodos_root: &Path, config_path: Option<&Path>) -> Result<Self> {
        let mut cfg = Config::default(neodos_root);

        // 1. Load global config (~/.config/neodev/neodev.toml)
        if let Ok(home) = env::var("HOME") {
            let global = PathBuf::from(home).join(".config/neodev/neodev.toml");
            if global.exists() {
                if let Ok(data) = std::fs::read_to_string(&global) {
                    if let Ok(file_cfg) = toml::from_str::<ConfigFile>(&data) {
                        cfg.merge_file(file_cfg);
                    }
                }
            }
        }

        // 2. Load project config (neodev.toml in neodos_root)
        let project_cfg = neodos_root.join("neodev.toml");
        if project_cfg.exists() {
            let data = std::fs::read_to_string(&project_cfg)
                .context("Failed to read neodev.toml from neodos_root")?;
            if let Ok(file_cfg) = toml::from_str::<ConfigFile>(&data) {
                cfg.merge_file(file_cfg);
            }
        }

        // 3. Load explicit config path (overrides all)
        if let Some(path) = config_path {
            if path.exists() {
                let data = std::fs::read_to_string(path)
                    .context("Failed to read explicit config file")?;
                if let Ok(file_cfg) = toml::from_str::<ConfigFile>(&data) {
                    cfg.merge_file(file_cfg);
                }
            }
        }

        // 4. Apply environment variable overrides
        if let Ok(val) = env::var("NEODOS_KVM") {
            cfg.qemu_kvm = val == "1" || val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("NEODOS_BACKEND") {
            cfg.vm_backend = val;
        }

        Ok(cfg)
    }

    pub fn default(neodos_root: &Path) -> Self {
        Self {
            neodos_root: neodos_root.to_path_buf(),
            esp_size_mb: 100,
            neodos_size_mb: 10,
            gpt_padding_mb: 12,
            ovmf_code: PathBuf::from("/usr/share/OVMF/OVMF_CODE.fd"),
            ovmf_vars_template: PathBuf::from("/usr/share/OVMF/OVMF_VARS.fd"),
            vm_backend: "qemu".into(),
            vm_memory_mb: 512,
            vm_cpus: 1,
            vm_network: "bridged".into(),
            qemu_kvm: false,
            qemu_bdm: false,
            kernel_target: "x86_64-unknown-none".into(),
            bootloader_target: "x86_64-unknown-uefi".into(),
            debug: false,
        }
    }

    fn merge_file(&mut self, file: ConfigFile) {
        if let Some(p) = file.project {
            if let Some(v) = p.esp_size_mb { self.esp_size_mb = v; }
            if let Some(v) = p.neodos_size_mb { self.neodos_size_mb = v; }
            if let Some(v) = p.gpt_padding_mb { self.gpt_padding_mb = v; }
            if let Some(v) = p.kernel_target { self.kernel_target = v; }
            if let Some(v) = p.bootloader_target { self.bootloader_target = v; }
            if let Some(v) = p.debug { self.debug = v; }
        }
        if let Some(v) = file.vm {
            if let Some(val) = v.backend { self.vm_backend = val; }
            if let Some(val) = v.memory { self.vm_memory_mb = val; }
            if let Some(val) = v.cpus { self.vm_cpus = val; }
            if let Some(val) = v.network { self.vm_network = val; }
        }
        if let Some(q) = file.qemu {
            if let Some(val) = q.kvm { self.qemu_kvm = val; }
            if let Some(val) = q.bdm { self.qemu_bdm = val; }
            if let Some(val) = q.ovmf_code { self.ovmf_code = val; }
            if let Some(val) = q.ovmf_vars_template { self.ovmf_vars_template = val; }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    project: Option<ProjectConfig>,
    vm: Option<VmConfig>,
    qemu: Option<QemuConfig>,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    esp_size_mb: Option<u64>,
    neodos_size_mb: Option<u64>,
    gpt_padding_mb: Option<u64>,
    kernel_target: Option<String>,
    bootloader_target: Option<String>,
    debug: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct VmConfig {
    backend: Option<String>,
    memory: Option<u32>,
    cpus: Option<u32>,
    network: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QemuConfig {
    kvm: Option<bool>,
    bdm: Option<bool>,
    ovmf_code: Option<PathBuf>,
    ovmf_vars_template: Option<PathBuf>,
}
