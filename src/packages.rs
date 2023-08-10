use destiny_pkg::PackageManager;
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    pub static PACKAGE_MANAGER: RefCell<Option<Arc<PackageManager>>> = RefCell::new(None);
}

pub fn package_manager_checked() -> anyhow::Result<Arc<PackageManager>> {
    PACKAGE_MANAGER.with(|v| {
        v.borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Package manager is not initialized!"))
    })
}

pub fn package_manager() -> Arc<PackageManager> {
    package_manager_checked().unwrap()
}
