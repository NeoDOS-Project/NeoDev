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

[virtualbox]
headless = false          # default display mode for the VirtualBox backend
vrde = false              # enable VirtualBox's RDP server (VRDE)
vrde_port = 5000
vrde_address = "0.0.0.0"
vrde_certificate = "/etc/vrde/server.crt"   # optional: RDP TLS certificate (PEM)
vrde_private_key = "/etc/vrde/server.key"   # optional: RDP TLS private key (PEM)
```

`vrde_certificate` and `vrde_private_key` must be set together; both are applied
as VirtualBox `Security/ServerCertificate` and `Security/ServerPrivateKey` VRDE
properties. Without them VRDE has no TLS certificate and modern RDP clients
reject the connection (`SSL_CERT_NOT_ON_SERVER`). Generate a self-signed pair
with `winpr-makecert -format crt -n "CN=NeoDOS" server`.

### How CLI flags combine with the config

- `--kvm` / `--bdm` only turn the feature **on**; the file value is used when
  the flag is absent (`NEODOS_KVM=1` also enables KVM).
- `--net` overrides `[vm] network`. When omitted, `neodev run` uses
  `[vm] network` (`user` by default).
- `--headless` accepts an optional boolean: `--headless` forces headless,
  `--headless=false` forces a GUI, and omitting it uses
  `[virtualbox] headless` for the VirtualBox backend (GUI for QEMU).
- `--backend` / `NEODOS_BACKEND` override `[vm] backend`.

## Environment Variables

| Variable | Overrides |
|----------|-----------|
| `NEODOS_PATH` | NeoDOS project root path |
| `NEODOS_BACKEND` | VM backend (`qemu` or `virtualbox`) |
| `NEODOS_KVM` | KVM acceleration (`1` or `true`) |
| `NEODOS_BRIDGE` | Bridge interface name for QEMU |
