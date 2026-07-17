pub mod abi;
mod backend;
pub mod loader;

use self::backend::PluginBackend;
use self::loader::discover_plugins;
use anyhow::Result;
use std::path::Path;

pub fn create_plugin_backends(plugin_dir: &Path) -> Vec<PluginBackend> {
    let loaded = discover_plugins(plugin_dir);
    loaded.into_iter().map(|l| PluginBackend::new(&l)).collect()
}

pub fn find_plugin_backend(plugin_dir: &Path, name: &str) -> Result<Option<PluginBackend>> {
    let loaded = discover_plugins(plugin_dir);
    for plugin in loaded {
        let backend = PluginBackend::new(&plugin);
        if backend.name == name {
            return Ok(Some(backend));
        }
    }
    Ok(None)
}
