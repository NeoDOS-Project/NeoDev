# NeoDev — NeoDOS Development Tool

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**NeoDev** is the official build, image, run, test, and CI toolchain for the
[NeoDOS](https://github.com/NeoDOS-Project/NeoDOS) operating system.

It replaces legacy shell scripts with a single Rust binary.

## Quick Start

```bash
# Install
cargo install --path .

# Point to NeoDOS and build
neodev --neodos-path /path/to/NeoDOS build --image

# Run in QEMU
neodev run

# Run tests
neodev test
```

## Commands

| Command | Description |
|---------|-------------|
| `neodev build` | Build kernel, bootloader, user binaries, NXL, NEM |
| `neodev build --quick` | Kernel + bootloader only |
| `neodev build --image` | Build and generate disk image |
| `neodev image` | Create NE2, ESP, and GPT disk image |
| `neodev run` | Run NeoDOS in VM (QEMU default) |
| `neodev run --kvm --gdb` | KVM acceleration + GDB server |
| `neodev test` | Run automated tests in headless VM |
| `neodev clean` | Remove build artifacts |
| `neodev list` | List discovered projects |
| `neodev config` | Show current configuration |
| `neodev vm start\|stop\|create\|delete` | Manage VMs |

## Configuration

NeoDev loads configuration from (in order):

1. `~/.config/neodev/neodev.toml` (global)
2. `<neodos_root>/neodev.toml` (project)
3. `--config <path>` (explicit override)

Example `neodev.toml`:

```toml
[project]
esp_size_mb = 100
kernel_target = "x86_64-unknown-none"
bootloader_target = "x86_64-unknown-uefi"

[vm]
backend = "qemu"
memory = 512
cpus = 2
network = "bridged"

[qemu]
kvm = false
ovmf_code = "/usr/share/OVMF/OVMF_CODE.fd"
ovmf_vars_template = "/usr/share/OVMF/OVMF_VARS.fd"
```

## Locating NeoDOS

NeoDev finds the NeoDOS project through (priority order):

1. `--neodos-path <path>` CLI flag
2. `NEODOS_PATH` environment variable
3. Auto-detection: walks up from current directory looking for `neodos-kernel/Cargo.toml`

## Backends

| Backend | Command | Requirements |
|---------|---------|-------------|
| QEMU (default) | `neodev run` | `qemu-system-x86_64`, OVMF |
| VirtualBox | `neodev run --backend virtualbox` | `VBoxManage` |

## Documentation

Full documentation in the `docs/` directory:

- [Architecture](docs/architecture.md)
- [Commands](docs/commands.md)
- [Configuration](docs/configuration.md)
- [Installation](docs/installation.md)
- [Integration with NeoDOS](docs/integration.md)

## License

MIT — see [LICENSE](LICENSE).
