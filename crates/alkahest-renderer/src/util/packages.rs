use destiny_pkg::TagHash;
use alkahest_pm::package_manager;

pub trait TagHashExt {
    fn prepend_package_name(&self) -> String;
}

impl TagHashExt for TagHash {
    fn prepend_package_name(&self) -> String {
        if let Some(path) = package_manager().package_paths.get(&self.pkg_id()) {
            format!("{}_{}/{}", path.name, path.id, self)
        } else {
            format!("unk/{}", self)
        }
    }
}
