# NeoDev Architecture

## Overview

NeoDev is a standalone CLI tool written in Rust that manages the NeoDOS development
lifecycle: compilation, disk image creation, VM execution, and automated testing.

```
            ┌─────────────────────┐
            │     neodev CLI      │
            │   (clap::Parser)    │
            └──────┬──────────────┘
                   │ dispatch
          ┌────────┼────────┬──────────┐
          ▼        ▼        ▼          ▼
     ┌────────┐┌────────┐┌────────┐┌──────────┐
     │ build  ││ image  ││  run   ││   test   │
     └────────┘└────────┘└────┬───┘└────┬─────┘
                              │         │
                              ▼         ▼
                       ┌──────────┐┌──────────┐
                       │ vmm/qemu ││vmm/vbox  │
                       │ QemuBknd ││VBoxBknd  │
                       └──────────┘└──────────┘
```

## Module Map

| Module | Responsibility |
|--------|---------------|
| `main.rs` | CLI definition (clap), command dispatch, NeoDOS path resolution |
| `config.rs` | Layered configuration loading (global → project → explicit → env) |
| `discovery.rs` | Auto-detect NeoDOS projects (kernel, bootloader, user bins, NXL, NEM, tools) |
| `build.rs` | Compile kernel, bootloader, user binaries, NXL libraries, NEM drivers, NLT files |
| `image.rs` | Generate NE2 filesystem, FAT32 ESP, unified GPT disk image |
| `run.rs` | Coordinate VM execution with selected backend |
| `test_.rs` | Run NeoTest suite in headless VM, parse serial output for results |
| `clean.rs` | Remove build artifacts |
| `report.rs` | Build/test result formatting |
| `vmm/mod.rs` | `HypervisorBackend` trait, `VmInstance` trait, factory, config helpers |
| `vmm/qemu.rs` | QEMU backend implementation |
| `vmm/vbox.rs` | VirtualBox backend implementation |

## Backend Architecture

NeoDev uses two traits for hypervisor abstraction:

- **`HypervisorBackend`**: lifecycle management (prerequisites, run, stop, reset, status)
- **`VmInstance`**: running VM handle (serial output, wait, kill)

Adding a new backend requires only implementing `HypervisorBackend` and
registering it in `create_backend()` — no CLI changes needed.

## Configuration Layering

```
~/.config/neodev/neodev.toml    (global defaults)
       ↓
<neodos_root>/neodev.toml       (project overrides)
       ↓
--config <path>                  (explicit overrides)
       ↓
NEODOS_KVM=1                    (environment overrides)
```

## NeoDOS Detection

NeoDev treats NeoDOS as an external project. It locates it via:

1. `--neodos-path` CLI flag
2. `NEODOS_PATH` environment variable
3. Walking up from CWD looking for `neodos-kernel/Cargo.toml`

All NeoDOS-specific paths (kernel, bootloader, userbin, drivers, etc.) are
resolved relative to this root.
