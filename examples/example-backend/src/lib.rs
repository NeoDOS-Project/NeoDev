use std::ffi::{c_char, c_int, c_void, CStr, CString};

#[repr(C)]
struct PluginVmConfig {
    name: *const c_char,
    memory_mb: u32,
    cpus: u32,
    efi: bool,
    disk_image: *const c_char,
    disk_vdi: *const c_char,
    serial_file: *const c_char,
    network: c_int,
    headless: bool,
    gdb: bool,
    gdb_port: u16,
    storage_mode: c_int,
}

#[repr(C)]
struct PluginConfig {
    neodos_root: *const c_char,
    esp_size_mb: u64,
    neodos_size_mb: u64,
    vm_memory_mb: u32,
    vm_cpus: u32,
    vm_network: *const c_char,
    qemu_kvm: bool,
    qemu_bdm: bool,
    ovmf_code: *const c_char,
    ovmf_vars_template: *const c_char,
    kernel_target: *const c_char,
    bootloader_target: *const c_char,
    debug: bool,
}

#[repr(C)]
enum PluginVmStatus {
    Running,
    Paused,
    Stopped,
    NotFound,
}

#[repr(C)]
struct PluginVmInstanceV1 {
    serial_path: Option<extern "C" fn(ctx: *mut c_void) -> *const c_char>,
    wait_timeout: Option<extern "C" fn(ctx: *mut c_void, timeout_ms: u64) -> c_int>,
    kill: Option<extern "C" fn(ctx: *mut c_void) -> c_int>,
    pid: Option<extern "C" fn(ctx: *mut c_void) -> u32>,
    destroy: Option<extern "C" fn(ctx: *mut c_void)>,
}

#[repr(C)]
struct NeodevBackendV1 {
    name: *const c_char,
    version: *const c_char,
    description: *const c_char,
    abi_version: u32,
    create: extern "C" fn() -> *mut c_void,
    destroy: extern "C" fn(*mut c_void),
    check_prerequisites: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig) -> c_int,
    ensure_vm: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> c_int,
    delete_vm: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> c_int,
    run: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> c_int,
    start_headless: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> *mut c_void,
    stop: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> c_int,
    reset: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> c_int,
    status: extern "C" fn(ctx: *mut c_void, cfg: *const PluginConfig, vmcfg: *const PluginVmConfig) -> PluginVmStatus,
}

struct ExampleState {
    name: CString,
}

extern "C" fn example_create() -> *mut c_void {
    let state = Box::new(ExampleState {
        name: CString::new("example").unwrap(),
    });
    Box::into_raw(state) as *mut c_void
}

extern "C" fn example_destroy(ctx: *mut c_void) {
    if !ctx.is_null() {
        unsafe { drop(Box::from_raw(ctx as *mut ExampleState)); }
    }
}

extern "C" fn example_check_prerequisites(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
) -> c_int {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    0
}

extern "C" fn example_ensure_vm(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    _vmcfg: *const PluginVmConfig,
) -> c_int {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    0
}

extern "C" fn example_delete_vm(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    _vmcfg: *const PluginVmConfig,
) -> c_int {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    0
}

extern "C" fn example_run(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    vmcfg: *const PluginVmConfig,
) -> c_int {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    let vmcfg = unsafe { &*vmcfg };
    let name = unsafe { CStr::from_ptr(vmcfg.name) }.to_str().unwrap_or("?");
    println!("[example-backend] run: {} ({} MB, {} CPUs)", name, vmcfg.memory_mb, vmcfg.cpus);
    0
}

extern "C" fn example_start_headless(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    _vmcfg: *const PluginVmConfig,
) -> *mut c_void {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    let instance = Box::new(ExampleInstance { _private: () });
    Box::into_raw(instance) as *mut c_void
}

extern "C" fn example_stop(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    _vmcfg: *const PluginVmConfig,
) -> c_int {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    0
}

extern "C" fn example_reset(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    _vmcfg: *const PluginVmConfig,
) -> c_int {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    0
}

extern "C" fn example_status(
    ctx: *mut c_void,
    _cfg: *const PluginConfig,
    _vmcfg: *const PluginVmConfig,
) -> PluginVmStatus {
    let _state = unsafe { &*(ctx as *const ExampleState) };
    PluginVmStatus::Stopped
}

struct ExampleInstance {
    _private: (),
}

extern "C" fn example_instance_serial_path(_ctx: *mut c_void) -> *const c_char {
    std::ptr::null()
}

extern "C" fn example_instance_wait_timeout(
    _ctx: *mut c_void,
    timeout_ms: u64,
) -> c_int {
    std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
    -1
}

extern "C" fn example_instance_kill(_ctx: *mut c_void) -> c_int {
    0
}

extern "C" fn example_instance_pid(_ctx: *mut c_void) -> u32 {
    0
}

extern "C" fn example_instance_destroy(ctx: *mut c_void) {
    if !ctx.is_null() {
        unsafe { drop(Box::from_raw(ctx as *mut ExampleInstance)); }
    }
}

unsafe impl Sync for PluginVmInstanceV1 {}

static INSTANCE_VTABLE: PluginVmInstanceV1 = PluginVmInstanceV1 {
    serial_path: Some(example_instance_serial_path),
    wait_timeout: Some(example_instance_wait_timeout),
    kill: Some(example_instance_kill),
    pid: Some(example_instance_pid),
    destroy: Some(example_instance_destroy),
};

unsafe impl Sync for NeodevBackendV1 {}

static BACKEND: NeodevBackendV1 = NeodevBackendV1 {
    name: c"example-backend".as_ptr(),
    version: c"0.1.0".as_ptr(),
    description: c"Example NeoDev VM backend plugin".as_ptr(),
    abi_version: 1,
    create: example_create,
    destroy: example_destroy,
    check_prerequisites: example_check_prerequisites,
    ensure_vm: example_ensure_vm,
    delete_vm: example_delete_vm,
    run: example_run,
    start_headless: example_start_headless,
    stop: example_stop,
    reset: example_reset,
    status: example_status,
};

#[no_mangle]
pub extern "C" fn neodev_backend_get_v1() -> *const NeodevBackendV1 {
    &BACKEND as *const NeodevBackendV1
}
