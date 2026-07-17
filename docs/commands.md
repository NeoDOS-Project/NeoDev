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
| `--neodos-size` | 100 | NeoDOS filesystem size in MB |

## Image

```text
neodev image [--output <path>] [--esp-size <MB>] [--neodos-size <MB>]
             [--blocks <N>] [--label <name>] [--no-build]
```

## Run

```text
neodev run [--storage ahci|ata|nvme|virtio] [--net user|tap|bridge]
           [--kvm] [--gdb] [--bdm] [--headless] [--serial <file>]
           [--backend qemu|virtualbox]
```

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
neodev vm start   [--backend qemu|virtualbox] [--headless] [--net nat|bridged]
neodev vm stop    [--backend qemu|virtualbox]
neodev vm reset   [--backend qemu|virtualbox]
neodev vm status  [--backend qemu|virtualbox]
neodev vm create  [--backend qemu|virtualbox] [--net nat|bridged]
neodev vm delete  [--backend qemu|virtualbox]
```

## Other

```text
neodev config     Show configuration
neodev list       Show discovered projects
neodev nxp [--all] [<name>]
```
