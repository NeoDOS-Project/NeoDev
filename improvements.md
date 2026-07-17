# Improvements Detected During Extraction

## Architecture

| ID | Description | Priority | Complexity | Architectural Impact |
|----|-------------|----------|------------|---------------------|
| IMP-1 | **Plugin system for backends**: dynamic loading of VM backends via shared library | Low | High | High — requires trait object refactoring, dynamic loading |
| IMP-2 | **Test result persistence**: save test results to JSON/YAML for CI consumption | Medium | Low | Low — new module, no existing changes |
| IMP-3 | **Parallel builds**: build kernel, bootloader, user bins concurrently | Medium | Medium | Medium — requires async/thread coordination |
| IMP-4 | **Layered config validation**: validate neodev.toml schema on load with descriptive errors | Low | Low | Low — better error messages |
| IMP-5 | **Cargo workspace inside NeoDev**: support for development sub-tools as workspace members | Low | Medium | Medium — workspace restructuring |

## Configuration

| ID | Description | Priority | Complexity | Architectural Impact |
|----|-------------|----------|------------|---------------------|
| IMP-6 | **Config profiles**: `--profile debug|release|test` to switch settings | Medium | Low | Low — new field in Config |
| IMP-7 | **Auto-install missing toolchain**: `rustup target add` if missing, install OVMF if absent | Medium | Medium | Low — script execution |
| IMP-8 | **XDG base directory support**: follow XDG spec for config/cache/data dirs | Low | Low | Low — path resolution change |

## CLI

| ID | Description | Priority | Complexity | Architectural Impact |
|----|-------------|----------|------------|---------------------|
| IMP-9 | **Shell completions**: generate auto-completion for bash/zsh/fish | Low | Low | Low — clap built-in support |
| IMP-10 | **Progress bars**: show build progress with spinners/progress bars | Medium | Medium | Low — new dependency (indicatif) |
| IMP-11 | **Colorized diffs**: show compilation warnings/errors with color | Low | Low | Low — existing colored usage |

## Testing

| ID | Description | Priority | Complexity | Architectural Impact |
|----|-------------|----------|------------|---------------------|
| IMP-12 | **Distributed testing**: run tests across multiple machines | Low | High | High — network communication layer |
| IMP-13 | **Test groups via CLI**: `neodev test --group memory` to run specific test groups | Medium | Medium | Medium — QEMU argument passing |
| IMP-14 | **VM snapshot for tests**: faster test iteration via VM snapshots | Medium | High | Medium — backend extension |

## CI/CD

| ID | Description | Priority | Complexity | Architectural Impact |
|----|-------------|----------|------------|---------------------|
| IMP-15 | **GitHub Actions integration**: official CI workflow templates | High | Low | Low — new documentation |
| IMP-16 | **Automatic releases**: `neodev release` to publish new NeoDOS version | Low | High | High — git/GitHub interaction |
| IMP-17 | **SDK management**: download and manage NeoDOS SDK versions | Medium | High | High — new subsystem |

## Developer Experience

| ID | Description | Priority | Complexity | Architectural Impact |
|----|-------------|----------|------------|---------------------|
| IMP-18 | **`neodev init`**: scaffold a new NeoDOS project | Low | Medium | Medium — new command |
| IMP-19 | **`neodev docs`**: generate/browse NeoDOS documentation | Low | Medium | Medium — new command |
| IMP-20 | **Wireless bridge detection**: improve VirtualBox bridged network interface selection | Low | Low | Low — vbox.rs enhancement |
