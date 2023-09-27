use crate::util::RwLock;
use destiny_pkg::PackageManager;
use lazy_static::lazy_static;
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
