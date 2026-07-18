use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

struct Subsystem {
    paths: Vec<&'static str>,
    allowed: Vec<&'static str>,
    forbidden: Vec<&'static str>,
}

fn subsystems() -> HashMap<&'static str, Subsystem> {
    let mut m: HashMap<&'static str, Subsystem> = HashMap::new();

    m.insert("hal", Subsystem {
        paths: vec!["hal/"],
        allowed: vec!["arch"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog", "virtio",
            "urn", "handle", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("arch", Subsystem {
        paths: vec!["arch/"],
        allowed: vec!["hal", "memory", "scheduler", "syscall", "panic_classification"],
        forbidden: vec![
            "input", "console", "graphics", "font",
            "drivers", "fs", "vfs", "buffer", "net", "cm",
            "object", "security", "irp", "eventbus",
            "interrupts", "timers", "nem", "apc", "dpc",
            "kwait", "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "handle", "elf",
            "nxl", "usermode", "globals", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("cpu", Subsystem {
        paths: vec!["cpu.rs"],
        allowed: vec!["arch", "hal", "memory"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog", "virtio",
            "urn", "handle", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("scheduler_boot", Subsystem {
        paths: vec!["scheduler/boot_"],
        allowed: vec!["arch", "memory", "hal", "scheduler", "object",
                       "security", "cm", "handle", "globals",
                       "usermode", "work_queue", "irp", "eventbus",
                       "dpc", "apc", "kwait", "panic_classification"],
        forbidden: vec![
            "input", "console", "graphics", "font",
            "drivers", "fs", "vfs", "buffer", "net",
            "interrupts", "timers", "virtio", "urn",
            "elf", "nxl", "crash", "debugger",
            "exception", "watchdog", "boot_benchmark",
            "abi_freeze",
        ],
    });

    m.insert("scheduler", Subsystem {
        paths: vec!["scheduler/"],
        allowed: vec!["arch", "memory", "hal", "scheduler_boot",
                       "object", "security", "cm", "handle",
                       "usermode", "work_queue", "irp",
                       "eventbus", "dpc", "apc", "kwait",
                       "panic_classification"],
        forbidden: vec![
            "input", "console", "graphics", "font",
            "drivers", "fs", "vfs", "buffer", "net",
            "interrupts", "timers", "virtio", "urn",
            "elf", "nxl", "crash", "debugger",
            "exception", "watchdog", "boot_benchmark",
            "abi_freeze",
        ],
    });

    m.insert("memory", Subsystem {
        paths: vec!["memory/"],
        allowed: vec!["arch", "hal", "scheduler", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "interrupts", "timers", "nem",
            "apc", "dpc", "kwait", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("memory_buddy", Subsystem {
        paths: vec!["memory/buddy"],
        allowed: vec!["arch", "hal", "memory", "panic_classification"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog", "virtio",
            "urn", "handle", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("memory_paging", Subsystem {
        paths: vec!["memory/paging"],
        allowed: vec!["arch", "hal", "memory", "panic_classification"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog", "virtio",
            "urn", "handle", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("memory_mmap", Subsystem {
        paths: vec!["memory/mmap"],
        allowed: vec!["arch", "hal", "memory", "scheduler",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "interrupts", "timers", "nem",
            "apc", "dpc", "kwait", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("input", Subsystem {
        paths: vec!["input/"],
        allowed: vec!["scheduler", "hal", "arch",
                       "eventbus", "object", "handle",
                       "console", "graphics", "font",
                       "irp", "dpc", "kwait"],
        forbidden: vec![
            "syscall", "memory", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "security", "timers",
            "interrupts", "nem", "apc", "crash",
            "debugger", "exception", "watchdog", "virtio",
            "urn", "elf", "nxl", "usermode",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("console", Subsystem {
        paths: vec!["console/"],
        allowed: vec!["scheduler", "hal", "arch", "input",
                       "graphics", "font", "eventbus",
                       "object", "handle", "usermode",
                       "irp", "dpc", "kwait"],
        forbidden: vec![
            "syscall", "memory", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "security", "timers",
            "interrupts", "nem", "apc", "crash",
            "debugger", "exception", "watchdog", "virtio",
            "urn", "elf", "nxl",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("graphics", Subsystem {
        paths: vec!["graphics/"],
        allowed: vec!["scheduler", "hal", "arch",
                       "font", "eventbus", "object",
                       "irp", "dpc", "handle", "usermode"],
        forbidden: vec![
            "syscall", "input", "console", "memory",
            "drivers", "fs", "vfs", "buffer", "net",
            "cm", "security", "timers", "interrupts",
            "nem", "apc", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "elf", "nxl",
            "globals", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("font", Subsystem {
        paths: vec!["font/"],
        allowed: vec!["scheduler", "hal", "arch"],
        forbidden: vec![
            "syscall", "input", "console", "memory",
            "graphics", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "timers",
            "interrupts", "nem", "apc", "dpc", "kwait",
            "crash", "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "globals", "work_queue",
            "irp", "eventbus", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("drivers", Subsystem {
        paths: vec!["drivers/"],
        allowed: vec!["arch", "hal", "memory", "scheduler",
                       "irp", "eventbus", "dpc", "kwait",
                       "handle", "object", "globals",
                       "console", "graphics", "font",
                       "input", "timer", "interrupts",
                       "usermode", "work_queue", "net",
                       "apc", "panic_classification",
                       "virtio", "urn", "cm", "security",
                       "drivers/isolation"],
        forbidden: vec![
            "syscall", "fs", "vfs", "buffer", "crash",
            "debugger", "exception", "watchdog",
            "elf", "nxl", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("fs", Subsystem {
        paths: vec!["fs/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "vfs", "globals", "irp", "eventbus",
                       "dpc", "object", "handle", "security",
                       "drivers/block", "crash", "usermode",
                       "cm", "kwait", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "buffer", "net", "timers",
            "interrupts", "nem", "apc", "drivers/fat32",
            "drivers/iso9660", "drivers/gpt",
            "drivers/storage_manager",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "elf", "nxl",
            "work_queue",
            "boot_benchmark", "abi_freeze", "drivers/ata",
            "drivers/boot_ahci", "drivers/nvme",
            "drivers/virtio_blk", "drivers/pci",
            "drivers/rtc_bridge", "drivers/ps2",
        ],
    });

    m.insert("vfs", Subsystem {
        paths: vec!["vfs/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "globals", "irp", "object", "handle",
                       "security", "drivers/block", "fs",
                       "usermode", "cm", "panic_classification",
                       "eventbus", "dpc", "kwait"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "buffer", "net", "timers",
            "interrupts", "nem", "apc", "drivers/fat32",
            "drivers/iso9660", "drivers/gpt",
            "drivers/storage_manager",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "elf", "nxl",
            "work_queue", "crash",
            "boot_benchmark", "abi_freeze", "drivers/ata",
            "drivers/boot_ahci", "drivers/nvme",
            "drivers/virtio_blk", "drivers/pci",
            "drivers/rtc_bridge", "drivers/ps2",
        ],
    });

    m.insert("buffer", Subsystem {
        paths: vec!["buffer/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "globals", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs",
            "net", "timers", "interrupts", "nem",
            "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "cm", "object", "security",
            "irp", "eventbus", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("net", Subsystem {
        paths: vec!["net/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "irp", "eventbus", "object", "handle",
                       "dpc", "globals", "kwait", "usermode",
                       "panic_classification",
                       "work_queue"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "cm", "security",
            "timers", "interrupts", "nem", "apc",
            "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "elf", "nxl",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("cm", Subsystem {
        paths: vec!["cm/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "object", "vfs", "fs", "globals",
                       "security", "handle", "usermode",
                       "irp", "eventbus", "dpc", "kwait",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "buffer", "net",
            "timers", "interrupts", "nem", "apc",
            "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("object", Subsystem {
        paths: vec!["object/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "console", "graphics", "font",
                       "globals", "handle", "irp",
                       "eventbus", "dpc", "kwait",
                       "usermode", "security", "net",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input",
            "drivers", "fs", "vfs", "buffer",
            "timers", "interrupts", "nem", "apc",
            "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "elf", "nxl",
            "work_queue", "cm",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("security", Subsystem {
        paths: vec!["security/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "object", "handle", "globals",
                       "usermode", "irp", "eventbus",
                       "dpc", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "timers", "interrupts",
            "nem", "apc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("irp", Subsystem {
        paths: vec!["irp/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "object", "handle", "kwait", "dpc",
                       "eventbus", "usermode", "globals"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "security", "timers",
            "interrupts", "nem", "apc", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "elf", "nxl",
            "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("eventbus", Subsystem {
        paths: vec!["eventbus/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "object", "irp", "dpc", "kwait",
                       "usermode", "globals"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "security", "timers",
            "interrupts", "nem", "apc", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("interrupts", Subsystem {
        paths: vec!["interrupts/"],
        allowed: vec!["arch", "hal", "memory", "scheduler",
                       "timers", "dpc", "eventbus",
                       "globals", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "nem", "apc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("timers", Subsystem {
        paths: vec!["timers/"],
        allowed: vec!["arch", "hal", "memory", "scheduler",
                       "interrupts", "dpc", "globals",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "nem", "apc", "kwait",
            "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "handle",
            "elf", "nxl", "usermode", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("nem", Subsystem {
        paths: vec!["nem/"],
        allowed: vec!["arch", "hal", "memory",
                       "panic_classification"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "globals", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("apc", Subsystem {
        paths: vec!["apc/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "irp", "dpc", "kwait", "object",
                       "handle", "eventbus", "globals",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "security", "timers",
            "interrupts", "nem", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "elf", "nxl", "usermode", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("dpc", Subsystem {
        paths: vec!["dpc/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "timers", "interrupts", "apc",
                       "irp", "eventbus", "kwait",
                       "globals", "object", "handle",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "security", "nem", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "elf", "nxl", "usermode",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("kwait", Subsystem {
        paths: vec!["kwait/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "dpc", "apc", "eventbus", "irp",
                       "globals", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "timers",
            "interrupts", "nem", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl", "usermode",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("crash", Subsystem {
        paths: vec!["crash/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "console", "graphics", "font",
                       "watchdog", "globals", "usermode",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input",
            "drivers", "fs", "vfs", "buffer", "net",
            "cm", "object", "security", "irp", "eventbus",
            "interrupts", "timers", "nem", "apc", "dpc",
            "kwait", "debugger", "exception",
            "virtio", "urn", "handle", "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("debugger", Subsystem {
        paths: vec!["debugger/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "crash", "console", "graphics",
                       "font", "globals", "usermode",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input",
            "drivers", "fs", "vfs", "buffer", "net",
            "cm", "object", "security", "irp", "eventbus",
            "interrupts", "timers", "nem", "apc", "dpc",
            "kwait", "exception", "watchdog", "virtio",
            "urn", "handle", "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("exception", Subsystem {
        paths: vec!["exception/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "usermode", "crash", "debugger",
                       "globals", "console", "graphics",
                       "font", "panic_classification"],
        forbidden: vec![
            "syscall", "input",
            "drivers", "fs", "vfs", "buffer", "net",
            "cm", "object", "security", "irp", "eventbus",
            "interrupts", "timers", "nem", "apc", "dpc",
            "kwait", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("watchdog", Subsystem {
        paths: vec!["watchdog/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "timers", "interrupts", "globals",
                       "crash", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "nem", "apc", "dpc", "kwait",
            "debugger", "exception", "virtio", "urn",
            "handle", "elf", "nxl", "usermode",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("virtio", Subsystem {
        paths: vec!["virtio/"],
        allowed: vec!["arch", "hal", "memory",
                       "interrupts", "timers",
                       "panic_classification"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "nem", "apc", "dpc",
            "kwait", "crash", "debugger", "exception",
            "watchdog", "urn", "handle", "elf", "nxl",
            "usermode", "globals", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("urn", Subsystem {
        paths: vec!["urn/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "irp", "eventbus", "object", "handle",
                       "dpc", "globals", "fs", "vfs",
                       "usermode", "cm", "kwait",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "buffer", "net",
            "security", "timers", "interrupts", "nem",
            "apc", "crash", "debugger", "exception",
            "watchdog", "virtio", "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("handle", Subsystem {
        paths: vec!["handle/"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "object", "globals",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "security", "irp", "eventbus",
            "interrupts", "timers", "nem", "apc", "dpc",
            "kwait", "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "elf", "nxl",
            "usermode", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("elf", Subsystem {
        paths: vec!["elf.rs"],
        allowed: vec!["arch", "hal", "memory"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "nxl",
            "usermode", "globals", "work_queue",
            "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("nxl", Subsystem {
        paths: vec!["nxl.rs"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "elf", "handle", "object", "globals",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "security", "irp", "eventbus",
            "interrupts", "timers", "nem", "apc", "dpc",
            "kwait", "crash", "debugger", "exception",
            "watchdog", "virtio", "urn",
            "usermode", "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("usermode", Subsystem {
        paths: vec!["usermode.rs"],
        allowed: vec!["arch", "scheduler", "memory", "hal"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "interrupts", "timers", "nem",
            "apc", "dpc", "kwait", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl",
            "globals", "work_queue",
            "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("globals", Subsystem {
        paths: vec!["globals.rs"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "vfs", "fs", "irp", "eventbus",
                       "object", "handle", "dpc", "kwait",
                       "cm", "security", "usermode",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "buffer", "net",
            "interrupts", "timers", "nem", "apc",
            "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "elf", "nxl",
            "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("work_queue", Subsystem {
        paths: vec!["work_queue.rs"],
        allowed: vec!["scheduler", "memory", "hal", "arch",
                       "dpc", "irp", "eventbus", "globals",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "timers",
            "interrupts", "nem", "apc", "kwait",
            "crash", "debugger", "exception",
            "watchdog", "virtio", "urn", "handle",
            "elf", "nxl", "usermode",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("panic_classification", Subsystem {
        paths: vec!["panic_classification.rs"],
        allowed: vec![],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "globals", "work_queue",
            "memory", "arch", "hal",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("boot_benchmark", Subsystem {
        paths: vec!["boot_benchmark.rs"],
        allowed: vec!["arch", "hal", "timers"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "nem",
            "apc", "dpc", "kwait", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl", "usermode",
            "globals", "work_queue",
            "panic_classification",
            "memory",
            "abi_freeze",
        ],
    });

    m.insert("abi_freeze", Subsystem {
        paths: vec!["abi_freeze.rs"],
        allowed: vec!["arch"],
        forbidden: vec![
            "scheduler", "syscall", "input", "console",
            "graphics", "font", "drivers", "fs", "vfs",
            "buffer", "net", "cm", "object", "security",
            "irp", "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait", "crash",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "globals", "work_queue",
            "panic_classification",
            "memory", "hal",
            "boot_benchmark",
        ],
    });

    m.insert("syscall", Subsystem {
        paths: vec!["syscall/"],
        allowed: vec![
            "scheduler", "input", "console", "hal", "arch",
            "vfs", "fs", "globals", "memory",
            "object", "security", "cm", "handle",
            "drivers/block", "irp", "eventbus",
            "graphics", "font", "usermode",
            "urn", "trace", "net",
        ],
        forbidden: vec![
            "drivers/ata", "drivers/boot_ahci", "drivers/fat32",
            "drivers/pci", "drivers/rtc_bridge", "drivers/nvme",
            "drivers/virtio_blk", "drivers/nem",
            "drivers/ps2", "drivers/gpt", "drivers/iso9660",
            "drivers/storage_manager", "drivers/driver_runtime",
            "drivers/hotreload", "drivers/isolation",
            "drivers/dependency", "drivers/boot_loader",
            "drivers/caps", "drivers/abi",
            "virtio", "crash", "debugger",
            "boot_benchmark", "abi_freeze", "invariants",
        ],
    });

    m.insert("usermode_shell", Subsystem {
        paths: vec!["usermode_shell.rs", "usermode_loader.rs"],
        allowed: vec!["scheduler", "memory", "usermode",
                       "arch", "hal", "globals"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "interrupts", "timers", "nem",
            "apc", "dpc", "kwait", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl",
            "work_queue", "panic_classification",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("input_extra", Subsystem {
        paths: vec!["kbd/"],
        allowed: vec!["scheduler", "hal", "arch",
                       "input", "console", "graphics",
                       "font", "eventbus", "object",
                       "handle", "irp", "dpc",
                       "cm", "usermode", "globals",
                       "panic_classification", "kwait"],
        forbidden: vec![
            "syscall",
            "memory", "drivers", "fs", "vfs", "buffer",
            "net", "security", "timers", "interrupts",
            "nem", "apc", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "elf", "nxl",
            "work_queue", "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("power", Subsystem {
        paths: vec!["power/"],
        allowed: vec!["arch", "hal", "memory", "scheduler",
                       "timers", "interrupts", "globals",
                       "cm", "object", "eventbus",
                       "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "security", "irp", "nem", "apc",
            "dpc", "kwait", "crash", "debugger",
            "exception", "watchdog", "virtio", "urn",
            "handle", "elf", "nxl", "usermode",
            "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m.insert("invariants", Subsystem {
        paths: vec!["invariants.rs"],
        allowed: vec!["arch", "hal", "scheduler", "memory",
                       "globals", "crash", "panic_classification"],
        forbidden: vec![
            "syscall", "input", "console", "graphics",
            "font", "drivers", "fs", "vfs", "buffer",
            "net", "cm", "object", "security", "irp",
            "eventbus", "interrupts", "timers",
            "nem", "apc", "dpc", "kwait",
            "debugger", "exception", "watchdog",
            "virtio", "urn", "handle", "elf", "nxl",
            "usermode", "work_queue",
            "boot_benchmark", "abi_freeze",
        ],
    });

    m
}

fn path_matches(pattern: &str, module_path: &str) -> bool {
    let pat = pattern.replace('/', "::");
    let escaped = regex::escape(&pat);
    let re_str = format!(r"(?:^|::){}(?:::|\Z)", escaped);
    Regex::new(&re_str).map_or(false, |re| re.is_match(module_path))
}

fn get_owning_subsystem<'a>(rel_path: &str, subs: &'a HashMap<&'static str, Subsystem>) -> Option<&'a str> {
    let rel = rel_path.replace('\\', "/");
    let mut candidates: Vec<(usize, &str)> = Vec::new();
    for (name, info) in subs {
        for pat in &info.paths {
            if rel.starts_with(pat) || rel == *pat {
                candidates.push((pat.len(), name));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.first().map(|&(_, name)| name)
}

fn extract_crate_imports(content: &str) -> Vec<String> {
    let re = Regex::new(r"^\s*use\s+(?:crate::)?([^;]+);").unwrap();
    content.lines().filter_map(|line| {
        re.captures(line).map(|c| c[1].to_string())
    }).collect()
}

fn check_forbidden(from_subsystem: &str, imported_path: &str, subs: &HashMap<&'static str, Subsystem>) -> Vec<&'static str> {
    let info = match subs.get(from_subsystem) {
        Some(i) => i,
        None => return vec![],
    };
    let mut violations = Vec::new();
    for forbidden in &info.forbidden {
        if path_matches(forbidden, imported_path) {
            let allowed = info.allowed.iter().any(|a| path_matches(a, imported_path));
            if !allowed {
                violations.push(*forbidden);
            }
        }
    }
    violations
}

pub fn run_check_deps(kernel_src: &Path) -> Result<()> {
    let subs = subsystems();
    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir::WalkDir::new(kernel_src)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let rel = path.strip_prefix(kernel_src)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let subsystem = match get_owning_subsystem(&rel, &subs) {
            Some(s) => s,
            None => continue,
        };

        let content = std::fs::read_to_string(path)?;
        let imports = extract_crate_imports(&content);

        for imp in &imports {
            let forbidden = check_forbidden(subsystem, imp, &subs);
            if !forbidden.is_empty() {
                violations.push(format!(
                    "  {}: imports '{}' which contains forbidden dep(s): {}",
                    rel, imp, forbidden.join(", ")
                ));
            }
        }
    }

    println!("{}", "=".repeat(60));
    println!("NeoDOS Dependency Check");
    println!("{}", "=".repeat(60));

    if violations.is_empty() {
        println!("\n\u{2705} No dependency violations found.");
    } else {
        println!("\n\u{274c} {} violation(s) found:\n", violations.len());
        for v in &violations {
            println!("{}", v);
        }
    }

    Ok(())
}
