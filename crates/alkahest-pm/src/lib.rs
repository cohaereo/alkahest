use destiny_pkg::{PackageManager, TagHash};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::sync::Arc;

lazy_static! {
    pub static ref PACKAGE_MANAGER: RwLock<Option<Arc<PackageManager>>> = RwLock::new(None);
}

pub fn package_manager_checked() -> anyhow::Result<Arc<PackageManager>> {
    PACKAGE_MANAGER
        .read()
        .as_ref()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Package manager is not initialized!"))
}

pub fn package_manager() -> Arc<PackageManager> {
    package_manager_checked().unwrap()
}

/// In case of multiple tags, the last one is returned
pub fn get_named_tag<const T: u32>(name: &str) -> Option<TagHash> {
    package_manager()
        .named_tags
        .iter()
        .rev()
        .find(|n| n.name == name && n.class_hash == T)
        .map(|n| &n.hash)
        .cloned()
}
