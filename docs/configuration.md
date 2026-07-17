# Configuration

NeoDev uses TOML configuration files. Settings are merged from multiple sources
with later sources overriding earlier ones.

## Config Files

### Global (`~/.config/neodev/neodev.toml`)

Applied for all projects.

### Project (`<neodos_root>/neodev.toml`)

Applied for a specific NeoDOS checkout.

### Explicit (`--config <path>`)

Applied on top of all others.

## Schema

```toml
[project]
esp_size_mb = 100
neodos_size_mb = 10
gpt_padding_mb = 12
kernel_target = "x86_64-unknown-none"
bootloader_target = "x86_64-unknown-uefi"
debug = false

[vm]
backend = "qemu"          # "qemu" or "virtualbox"
memory = 512              # MB
cpus = 2
network = "bridged"       # "user", "bridged", or "none"

[qemu]
kvm = false
bdm = false               # persistent OVMF vars
ovmf_code = "/usr/share/OVMF/OVMF_CODE.fd"
ovmf_vars_template = "/usr/share/OVMF/OVMF_VARS.fd"
```

## Environment Variables

| Variable | Overrides |
|----------|-----------|
| `NEODOS_PATH` | NeoDOS project root path |
| `NEODOS_BACKEND` | VM backend (`qemu` or `virtualbox`) |
| `NEODOS_KVM` | KVM acceleration (`1` or `true`) |
| `NEODOS_BRIDGE` | Bridge interface name for QEMU |
