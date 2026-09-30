# Changelog

## Unreleased

### Fixed

- **`netapplier.nxe` is now installed into `/System/Tools`.** The network
  configuration applier service (`NeoDOS-Project/NeoDOS#365`) is a new Ring 3
  service at `C:\System\Tools\netapplier.nxe`. `collect_files()` has a hardcoded
  user-binary list, so without this entry the built binary was silently omitted
  from the NE2 image and the `NetApplier` service could not start.

## v0.3.0 (2026-09-27)

### Fixed

- **VirtualBox stale VDI (issue #1)**: `neodev test --backend virtualbox`
  never refreshed `disk_image.vdi`, so it could boot the previous kernel even
  after `neodev build --quick --image` produced a newer `disk_image.img`.
  - New single authoritative path `vbox::ensure_vdi_current()` owns the
    `raw image -> VDI` freshness policy and is now called by both
    `ensure_vm()` (`run`) and `start_headless()` (`test`, `dhcp`).
  - Deterministic rule: regenerate when the VDI is missing or when
    `mtime(img) > mtime(vdi)`; equal timestamps mean current (no sleep/fudge
    factor).
  - The medium is detached before replacement and re-attached afterwards, so an
    attached VDI is never blindly deleted or overwritten. VM name, firmware,
    chipset, AHCI controller, port/device, MAC and network config are preserved.
  - A running/paused VM is never modified; regeneration fails with a clear
    error instead. Missing `disk_image.img` and `VBoxManage convertfromraw`
    failures return actionable errors (source, destination, exit status,
    output) instead of silently booting a stale VDI.
  - Post-conversion verification: the VDI must exist and be at least as new as
    the raw image.
  - Regression tests cover missing VDI, newer/equal/stale VDI, missing IMG,
    conversion failure and paths containing spaces.

### Changed

- Removed every compiler and Clippy warning across the crate: simplified
  iterator/`map_or`/sort patterns, used `div_ceil`, removed dead struct fields
  and unused parameters, and moved the test module to the end of its file.
- Bumped to `0.3.0`.

### Added

- `scripts/` tooling for VirtualBox validation (reused from local harnesses):
  `vbox-vdi-sync-check.sh` and `vbox-boot-probe.py`.

## v0.2.1 (2026-09-26)

### Fixed

- **NE2 directory leaf overflow**: `make_btree_leaf()` could declare more entries
  than it serialized when a directory exceeded a single leaf, producing a
  structurally inconsistent leaf and silently dropping entries.
  - `/Programs` (34 entries) overflowed the ~28-entry capacity: `reboot`,
    `shtest`, `stresscmd`, `tree`, `ver`, `vol` were lost, and `ping`/`ps`
    lookups failed.
  - `make_btree_leaf()` now fails fast with a diagnostic (directory, requested
    count, capacity) instead of truncating.
  - Rebalanced `programs_nxe`/`tools_nxe` so every generated leaf fits
    (all programs remain present; `System/Tools` is on the shell PATH).
  - Added regression tests: leaf at capacity succeeds; over-capacity fails
    explicitly; declared count always equals serialized count.
- **Explicit-flag build used a 10 MB image**: `build --kernel/--userbin/...`
  passed a hardcoded `2560` NE2 blocks instead of the configured
  `--neodos-blocks` (default 25600 = 100 MB).  This produced a too-small image
  that could panic the kernel at boot.  Now it uses `neodos_blocks`, matching
  the `--all` and `--quick` paths.
- **QEMU ignored the configured CPU count**: `run` and `start_headless` never
  passed `-smp`, so `neodev run`/`neodev test` always booted a single vCPU
  regardless of `[vm] cpus`.  This forced `netd` onto the BSP and starved
  userland.  Both paths now honor `vmcfg.cpus`.

## v0.2.0 (2025-07-17)

### Major

- **Standalone repository**: NeoDev extracted from NeoDOS to `github.com/NeoDOS-Project/NeoDev`
- **Configurable NeoDOS path**: `--neodos-path` CLI flag, `NEODOS_PATH` env var, or auto-detect
- **Multi-source configuration**: layered loading from `~/.config/neodev/neodev.toml`, project `neodev.toml`, and explicit `--config`
- **CLI aliases**: `build` → `b`, `image` → `i`, `run` → `r`, `test` → `t`, `clean` → `c`, `config` → `cfg`, `list` → `ls`

### Changed

- All hardcoded NeoDOS paths replaced with configurable fields
- `project_root` renamed to `neodos_root` across the codebase
- Configuration loading completely rewritten for multi-source merge
- Improved error messages for missing NeoDOS project root

### Removed

- Internal dependency on NeoDOS directory structure
- Hardcoded references to `tools/neodev/` path within NeoDOS
- Legacy `find_project_root()` replaced with `resolve_neodos_root()`

## v0.1.0 (2025-04-01)

- Initial release (inside NeoDOS repository)
- Build, image, run, test, clean, and VM management commands
- QEMU and VirtualBox backends
- NE2 filesystem and GPT image generation
