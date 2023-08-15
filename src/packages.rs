use destiny_pkg::PackageManager;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    pub static PACKAGE_MANAGER: RefCell<Option<Rc<PackageManager>>> = RefCell::new(None);
}

pub fn package_manager_checked() -> anyhow::Result<Rc<PackageManager>> {
    PACKAGE_MANAGER.with(|v| {
        v.borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Package manager is not initialized!"))
    })
}

pub fn package_manager() -> Rc<PackageManager> {
    package_manager_checked().unwrap()
}
