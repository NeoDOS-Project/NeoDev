use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct PluginVmConfig {
    pub name: *const c_char,
    pub memory_mb: u32,
    pub cpus: u32,
    pub efi: bool,
    pub disk_image: *const c_char,
    pub disk_vdi: *const c_char,
    pub serial_file: *const c_char,
    pub network: c_int,
    pub headless: bool,
    pub gdb: bool,
    pub gdb_port: u16,
    pub storage_mode: c_int,
}

#[repr(C)]
pub struct PluginConfig {
    pub neodos_root: *const c_char,
    pub esp_size_mb: u64,
    pub neodos_size_mb: u64,
    pub vm_memory_mb: u32,
    pub vm_cpus: u32,
    pub vm_network: *const c_char,
    pub qemu_kvm: bool,
    pub qemu_bdm: bool,
    pub ovmf_code: *const c_char,
    pub ovmf_vars_template: *const c_char,
    pub kernel_target: *const c_char,
    pub bootloader_target: *const c_char,
    pub debug: bool,
}

#[repr(C)]
pub enum PluginVmStatus {
    Running,
    Paused,
    Stopped,
    NotFound,
}

#[repr(C)]
pub struct PluginVmInstanceV1 {
    pub serial_path: Option<extern "C" fn(ctx: *mut c_void) -> *const c_char>,
    pub wait_timeout: Option<extern "C" fn(ctx: *mut c_void, timeout_ms: u64) -> c_int>,
    pub kill: Option<extern "C" fn(ctx: *mut c_void) -> c_int>,
    pub pid: Option<extern "C" fn(ctx: *mut c_void) -> u32>,
    pub destroy: Option<extern "C" fn(ctx: *mut c_void)>,
}

pub type PluginResult = c_int;

#[repr(C)]
pub struct NeodevBackendV1 {
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub abi_version: u32,

    pub create: extern "C" fn() -> *mut c_void,
    pub destroy: extern "C" fn(*mut c_void),

    pub check_prerequisites: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig) -> PluginResult,
    pub ensure_vm: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginResult,
    pub delete_vm: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginResult,
    pub run: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginResult,
    pub start_headless: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> *mut c_void,
    pub stop: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginResult,
    pub reset: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginResult,
    pub status: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginVmStatus,
}

pub const NEODEV_BACKEND_ABI_V1: u32 = 1;
pub const NEODEV_PLUGIN_EXPORT: &[u8] = b"neodev_backend_get_v1\0";
