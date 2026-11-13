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
    pub vbox_headless: bool,
    pub vbox_vrde: bool,
    pub vbox_vrde_port: u16,
    pub vbox_vrde_address: String,
    pub vbox_vrde_certificate: Option<PathBuf>,
    pub vbox_vrde_private_key: Option<PathBuf>,
}

impl Config {
    pub fn load(neodos_root: &Path, config_path: Option<&Path>) -> Result<Self> {
        let mut cfg = Config::default(neodos_root);

        // 1. Load global config (~/.config/neodev/neodev.toml)
        if let Ok(home) = env::var("HOME") {
            let global = PathBuf::from(home).join(".config/neodev/neodev.toml");
            if global.exists() {
                cfg.merge_file(read_config_file(&global)?);
            }
        }

        // 2. Load project config (neodev.toml in neodos_root)
        let project_cfg = neodos_root.join("neodev.toml");
        if project_cfg.exists() {
            cfg.merge_file(read_config_file(&project_cfg)?);
        }

        // 3. Load explicit config path (overrides all). A path that was
        // explicitly requested but is missing is a user error, not a silent
        // fallback to defaults.
        if let Some(path) = config_path {
            if !path.exists() {
                anyhow::bail!("Config file not found: {}", path.display());
            }
            cfg.merge_file(read_config_file(path)?);
        }

        // 4. Apply environment variable overrides
        if let Ok(val) = env::var("NEODOS_KVM") {
            cfg.qemu_kvm = val == "1" || val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("NEODOS_BACKEND") {
            cfg.vm_backend = val;
        }

        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        const NETWORKS: &[&str] = &["user", "nat", "tap", "bridge", "bridged", "none", "disabled"];
        let network = self.vm_network.to_ascii_lowercase();
        if !NETWORKS.contains(&network.as_str()) {
            anyhow::bail!(
                "Invalid [vm] network '{}'. Use: user, bridged, none",
                self.vm_network
            );
        }
        Ok(())
    }

    pub fn default(neodos_root: &Path) -> Self {
        Self {
            neodos_root: neodos_root.to_path_buf(),
            esp_size_mb: 100,
            neodos_size_mb: 100,
            gpt_padding_mb: 12,
            ovmf_code: PathBuf::from("/usr/share/OVMF/OVMF_CODE.fd"),
            ovmf_vars_template: PathBuf::from("/usr/share/OVMF/OVMF_VARS.fd"),
            vm_backend: "qemu".into(),
            vm_memory_mb: 512,
            vm_cpus: 1,
            vm_network: "user".into(),
            qemu_kvm: false,
            qemu_bdm: false,
            kernel_target: "x86_64-unknown-none".into(),
            bootloader_target: "x86_64-unknown-uefi".into(),
            debug: false,
            vbox_headless: false,
            vbox_vrde: false,
            vbox_vrde_port: 5000,
            vbox_vrde_address: "0.0.0.0".into(),
            vbox_vrde_certificate: None,
            vbox_vrde_private_key: None,
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
        if let Some(v) = file.virtualbox {
            if let Some(val) = v.headless { self.vbox_headless = val; }
            if let Some(val) = v.vrde { self.vbox_vrde = val; }
            if let Some(val) = v.vrde_port { self.vbox_vrde_port = val; }
            if let Some(val) = v.vrde_address { self.vbox_vrde_address = val; }
            if let Some(val) = v.vrde_certificate { self.vbox_vrde_certificate = Some(val); }
            if let Some(val) = v.vrde_private_key { self.vbox_vrde_private_key = Some(val); }
        }
    }
}

fn read_config_file(path: &Path) -> Result<ConfigFile> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?;
    toml::from_str(&data)
        .with_context(|| format!("Failed to parse config file {}", path.display()))
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    project: Option<ProjectConfig>,
    vm: Option<VmConfig>,
    qemu: Option<QemuConfig>,
    virtualbox: Option<VirtualBoxConfig>,
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

#[derive(Debug, Deserialize)]
struct VirtualBoxConfig {
    headless: Option<bool>,
    vrde: Option<bool>,
    vrde_port: Option<u16>,
    vrde_address: Option<String>,
    vrde_certificate: Option<PathBuf>,
    vrde_private_key: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merged(toml_str: &str) -> Config {
        let mut cfg = Config::default(Path::new("/tmp/neodos"));
        let file: ConfigFile = toml::from_str(toml_str).expect("valid TOML");
        cfg.merge_file(file);
        cfg
    }

    #[test]
    fn virtualbox_section_is_parsed() {
        let cfg = merged(
            r#"
            [virtualbox]
            headless = true
            vrde = true
            vrde_port = 5001
            vrde_address = "127.0.0.1"
            vrde_certificate = "/etc/vrde/server.crt"
            vrde_private_key = "/etc/vrde/server.key"
            "#,
        );
        assert!(cfg.vbox_headless);
        assert!(cfg.vbox_vrde);
        assert_eq!(cfg.vbox_vrde_port, 5001);
        assert_eq!(cfg.vbox_vrde_address, "127.0.0.1");
        assert_eq!(
            cfg.vbox_vrde_certificate.as_deref(),
            Some(Path::new("/etc/vrde/server.crt"))
        );
        assert_eq!(
            cfg.vbox_vrde_private_key.as_deref(),
            Some(Path::new("/etc/vrde/server.key"))
        );
    }

    #[test]
    fn virtualbox_defaults_are_preserved_when_section_missing() {
        let cfg = merged("[vm]\nbackend = \"virtualbox\"\n");
        assert!(!cfg.vbox_headless);
        assert!(!cfg.vbox_vrde);
        assert_eq!(cfg.vbox_vrde_port, 5000);
        assert_eq!(cfg.vbox_vrde_address, "0.0.0.0");
        assert_eq!(cfg.vbox_vrde_certificate, None);
        assert_eq!(cfg.vbox_vrde_private_key, None);
    }

    #[test]
    fn vm_section_overrides_defaults() {
        let cfg = merged(
            r#"
            [vm]
            backend = "virtualbox"
            memory = 1024
            cpus = 4
            network = "user"
            "#,
        );
        assert_eq!(cfg.vm_backend, "virtualbox");
        assert_eq!(cfg.vm_memory_mb, 1024);
        assert_eq!(cfg.vm_cpus, 4);
        assert_eq!(cfg.vm_network, "user");
    }

    #[test]
    fn defaults_use_user_networking_and_a_100_mb_filesystem() {
        let cfg = Config::default(Path::new("/tmp/neodos"));
        assert_eq!(cfg.vm_network, "user");
        assert_eq!(cfg.neodos_size_mb, 100);
    }

    #[test]
    fn invalid_network_is_rejected() {
        let mut cfg = Config::default(Path::new("/tmp/neodos"));
        cfg.vm_network = "briged".into();
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("Invalid [vm] network"));
    }

    #[test]
    fn malformed_config_reports_the_file_path() {
        let dir = std::env::temp_dir().join(format!(
            "neodev-config-test-{}-malformed",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("broken.toml");
        std::fs::write(&path, "[vm\nnetwork =").unwrap();

        let err = read_config_file(&path).unwrap_err().to_string();
        assert!(err.contains("Failed to parse config file"));
        assert!(err.contains("broken.toml"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_explicit_config_is_an_error() {
        let dir = std::env::temp_dir().join(format!(
            "neodev-config-test-{}-missing",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("does-not-exist.toml");

        let err = Config::load(&dir, Some(&missing)).unwrap_err().to_string();
        assert!(err.contains("Config file not found"));
        assert!(err.contains("does-not-exist.toml"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
