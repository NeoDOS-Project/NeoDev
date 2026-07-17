# Integration with NeoDOS

## Overview

NeoDev treats NeoDOS as an external project. It locates the NeoDOS checkout
via `--neodos-path`, `NEODOS_PATH`, or auto-detection.

## Directory Structure Expected

```
<neodos_root>/
├── neodos-kernel/         # Kernel project
├── neodos-bootloader/     # EFI bootloader
├── userbin/               # User-mode NXE binaries
├── drivers/               # NEM kernel drivers
├── lib*-nxl/              # NXL shared libraries
├── tools/                 # Other tools (nltc, nxpkg, etc.)
├── scripts/               # Registry generation, etc.
├── data/                  # Locales, keyboard layouts
├── neodev.toml            # Project config (optional)
```

## Suggested Workflow

```bash
# Clone both repos
git clone https://github.com/NeoDOS-Project/NeoDOS.git
git clone https://github.com/NeoDOS-Project/NeoDev.git

# Build and install NeoDev
cd NeoDev && cargo build --release

# Use NeoDev from anywhere
cd ../NeoDOS
neodev build --image
neodev run
```

## CI Integration

```yaml
# .github/workflows/ci.yml
jobs:
  build:
    steps:
      - uses: actions/checkout@v4
        with:
          repository: NeoDOS-Project/NeoDOS
      - uses: actions/checkout@v4
        with:
          repository: NeoDOS-Project/NeoDev
          path: tools/neodev
      - run: cargo build --release --manifest-path tools/neodev/Cargo.toml
      - run: ./tools/neodev/target/release/neodev build --quick
      - run: ./tools/neodev/target/release/neodev test
```

## Backward Compatibility

NeoDev 0.2+ maintains full backward compatibility with NeoDOS. The existing
`neodev.toml` files in NeoDOS project roots are still read and respected.

The only change is that `neodev` is now invoked directly instead of via
`cargo run --manifest-path tools/neodev/Cargo.toml`.
