# NeoDev Commands

## Global Options

| Flag | Description |
|------|-------------|
| `--neodos-path <path>` | Path to NeoDOS project root |
| `--config <path>` | Path to explicit config file |
| `--help` | Show help |
| `--version` | Show version |

## Build

```text
neodev build [--kernel] [--bootloader] [--userbin] [--nxl] [--nem]
             [--all] [--quick] [--image] [--neodos-size <MB>]
             [--neodos-blocks <N>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--kernel` | false | Build kernel only |
| `--bootloader` | false | Build bootloader only |
| `--userbin` | false | Build user binaries |
| `--nxl` | false | Build NXL libraries |
| `--nem` | false | Build NEM drivers |
| `--all` | false | Build all (default if no flags) |
| `--quick` | false | Kernel + bootloader only |
| `--image` | false | Generate disk image after build |
| `--neodos-size` | `[project] neodos_size_mb` | NeoDOS partition size in MB |
| `--neodos-blocks` | `neodos_size * 256` | NE2 filesystem blocks (4 KB each) |

## Image

```text
neodev image [--output <path>] [--esp-size <MB>] [--neodos-size <MB>]
             [--blocks <N>] [--label <name>] [--no-build]
```

`--esp-size`, `--neodos-size` and `--blocks` default to the project
configuration. When `--blocks` is omitted it is derived from `--neodos-size`
(256 blocks per MB), so the filesystem and partition stay in sync.

## Run

```text
neodev run [--storage ahci|ata|nvme|virtio] [--net user|tap|bridge|none]
           [--kvm] [--gdb] [--bdm] [--headless[=true|false]] [--serial <file>]
           [--backend qemu|virtualbox]
```

`--net` defaults to `[vm] network`; `--headless` defaults to
`[virtualbox] headless` for VirtualBox and GUI for QEMU.

## RDP / Remmina

```text
neodev rdp [--host <host>] [--port <port>] [--user <user>] [--password <pass>]
           [--width <px>] [--height <px>]
           [--client auto|sdl-freerdp|xfreerdp|remmina-flatpak|<path>]
           [--print]
```

Connects to the VM's VirtualBox VRDE endpoint. Run it on the machine with the
display, not on a headless VM host. The host defaults to
`[virtualbox] vrde_address` (wildcard binds map to `127.0.0.1`) and the port to
`[virtualbox] vrde_port`.

Client selection order with `--client auto` is `sdl-freerdp`, the Remmina
Flatpak (`flatpak run --command=sdl-freerdp org.remmina.Remmina`), then
`xfreerdp`. `sdl-freerdp` is recommended because it sends the scancodes NeoDOS
expects. `--print` prints the resolved command without launching it, which is
useful from a headless host.

## Test

```text
neodev test [--storage ahci|ata|virtio] [--kvm]
            [--iterations <N>] [--timeout <sec>]
            [--backend qemu|virtualbox]
```

## DHCP Test

```text
neodev dhcp [--backend qemu|virtualbox] [--timeout <sec>]
```

## Clean

```text
neodev clean [--all] [--kernel] [--bootloader] [--userbin]
             [--nxl] [--nem] [--images]
```

## VM Management

```text
neodev vm start   [--backend qemu|virtualbox] [--headless[=true|false]] [--net user|bridged|none]
neodev vm stop    [--backend qemu|virtualbox]
neodev vm reset   [--backend qemu|virtualbox]
neodev vm status  [--backend qemu|virtualbox]
neodev vm create  [--backend qemu|virtualbox] [--net user|bridged|none]
neodev vm delete  [--backend qemu|virtualbox]
```

## Other

```text
neodev config     Show configuration
neodev list       Show discovered projects
neodev nxp [--all] [<name>]
```
