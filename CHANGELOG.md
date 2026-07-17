# Changelog

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
