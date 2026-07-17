use crate::config::Config;
use crate::vmm;
use anyhow::Result;
use colored::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageMode {
    Ahci, Ata, Nvme, Virtio,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetMode {
    User, Tap, Bridge,
}

pub struct RunOptions {
    pub storage: StorageMode,
    pub net: NetMode,
    pub kvm: bool,
    pub gdb: bool,
    pub headless: bool,
    pub bdm: bool,
    pub serial_file: Option<String>,
    pub backend: String,
}

pub fn run_vm(cfg: &Config, opts: &RunOptions) -> Result<()> {
    let backend_name = &opts.backend;
    let backend = vmm::create_backend(backend_name)?;

    println!("{} NeoDOS Session (backend: {})", "[*]".bold().cyan(), backend.name());
    println!();

    backend.check_prerequisites(cfg)?;

    let vmcfg = vmm::VmConfig {
        name: "NeoDOS".into(),
        memory_mb: cfg.vm_memory_mb,
        cpus: cfg.vm_cpus,
        efi: true,
        disk_image: cfg.neodos_root.join("disk_image.img"),
        disk_vdi: cfg.neodos_root.join("disk_image.vdi"),
        serial_file: opts.serial_file.clone().map(std::path::PathBuf::from),
        network: match opts.net {
            NetMode::User => vmm::NetworkMode::User,
            NetMode::Tap | NetMode::Bridge => vmm::NetworkMode::Bridged,
        },
        headless: opts.headless,
        gdb: opts.gdb,
        gdb_port: 1234,
        storage_mode: match opts.storage {
            StorageMode::Ahci => vmm::StorageMode::Ahci,
            StorageMode::Ata => vmm::StorageMode::Ata,
            StorageMode::Nvme => vmm::StorageMode::Nvme,
            StorageMode::Virtio => vmm::StorageMode::Virtio,
        },
    };

    backend.run(cfg, &vmcfg)
}
