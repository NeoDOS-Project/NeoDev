use super::abi::{NeodevBackendV1, NEODEV_BACKEND_ABI_V1, NEODEV_PLUGIN_EXPORT};
use anyhow::{Context, Result};
use libloading::Library;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct LoadedPlugin {
    pub backend: &'static NeodevBackendV1,
    _library: Arc<Library>,
    pub path: PathBuf,
}

fn find_plugin_libraries(plugin_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if !plugin_dir.exists() {
        return found;
    }
    let entries = match std::fs::read_dir(plugin_dir) {
        Ok(e) => e,
        Err(_) => return found,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_plugin = path.extension().and_then(|e| e.to_str()).map_or(false, |ext| {
            matches!(ext, "so" | "dylib" | "dll")
        });
        if is_plugin && path.is_file() {
            found.push(path);
        }
    }
    found.sort();
    found
}

pub fn discover_plugins(plugin_dir: &Path) -> Vec<LoadedPlugin> {
    let mut plugins = Vec::new();
    for path in find_plugin_libraries(plugin_dir) {
        match load_plugin(&path) {
            Ok(plugin) => {
                plugins.push(plugin);
            }
            Err(e) => {
                eprintln!("  [!] Failed to load plugin '{}': {}", path.display(), e);
            }
        }
    }
    plugins
}

pub fn load_plugin(path: &Path) -> Result<LoadedPlugin> {
    let lib = unsafe { Library::new(path) }.context("Failed to open shared library")?;
    let lib = Arc::new(lib);

    let get_backend: libloading::Symbol<unsafe extern "C" fn() -> *const NeodevBackendV1> = unsafe {
        lib.get(NEODEV_PLUGIN_EXPORT)
            .context("Plugin missing 'neodev_backend_get_v1' export")?
    };

    let backend_ptr = unsafe { get_backend() };
    if backend_ptr.is_null() {
        anyhow::bail!("Plugin returned null backend pointer");
    }

    let backend = unsafe { &*backend_ptr };

    if backend.abi_version != NEODEV_BACKEND_ABI_V1 {
        anyhow::bail!(
            "Plugin ABI version {} != expected {}",
            backend.abi_version,
            NEODEV_BACKEND_ABI_V1
        );
    }

    let name = unsafe { CStr::from_ptr(backend.name) }
        .to_str()
        .context("Plugin name is not valid UTF-8")?
        .to_owned();

    eprintln!("  [+] Loaded plugin: {} v{} ({})", name, {
        let v = unsafe { CStr::from_ptr(backend.version) }.to_str().unwrap_or("?");
        v
    }, path.display());

    Ok(LoadedPlugin {
        backend,
        _library: lib,
        path: path.to_path_buf(),
    })
}
