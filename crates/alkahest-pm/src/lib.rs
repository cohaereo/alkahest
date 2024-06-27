use std::sync::Arc;

use destiny_pkg::{PackageManager, TagHash};
use lazy_static::lazy_static;
use parking_lot::RwLock;

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

pub fn is_pkg_redacted(tag: impl Into<TagHash>) -> bool {
    let tag = tag.into();

    if let Some(path) = package_manager().package_paths.get(&tag.pkg_id()) {
        path.name.ends_with("redacted")
    } else {
        false
    }
}
