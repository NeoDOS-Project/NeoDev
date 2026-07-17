# Installation

## From Source

### Prerequisites

- Rust toolchain (edition 2021 or later)
- `nightly` Rust channel (for kernel compilation)
- x86_64 targets:

```bash
rustup target add x86_64-unknown-none
rustup target add x86_64-unknown-uefi
```

### Build and Install

```bash
git clone https://github.com/NeoDOS-Project/NeoDev.git
cd NeoDev
cargo build --release
cp target/release/neodev ~/.local/bin/   # or any PATH directory
```

### Using Cargo Install

```bash
cargo install --git https://github.com/NeoDOS-Project/NeoDev.git
```

## Runtime Dependencies

### For Building

- `cargo` with nightly toolchain
- `python3` (for NEM driver builds and registry scripts)

### For Running in QEMU

- `qemu-system-x86_64`
- OVMF firmware (`edk2-ovmf` / `ovmf` package)
- `mkfs.fat` (from `dosfstools`)
- `mtools` (for ESP file copying)

### For Running in VirtualBox

- `VirtualBox` with `VBoxManage` in PATH

### For Disk Images

- `sfdisk` (from `util-linux`)
- `dd`

## Quick Test

```bash
neodev --neodos-path /path/to/NeoDOS build --quick --image
neodev run
```
