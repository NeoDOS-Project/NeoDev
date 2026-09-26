# Changelog

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
