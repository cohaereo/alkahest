use alkahest_pm::package_manager;
use destiny_pkg::TagHash;

pub trait TagHashExt {
    fn prepend_package_name(&self) -> String;

    /// Does the pkg referenced by this tag exist?
    fn is_valid_pkg(&self) -> bool;
}

impl TagHashExt for TagHash {
    fn prepend_package_name(&self) -> String {
        if let Some(path) = package_manager().package_paths.get(&self.pkg_id()) {
            format!("{}_{}/{}", path.name, path.id, self)
        } else {
            format!("unk/{}", self)
        }
    }

    fn is_valid_pkg(&self) -> bool {
        package_manager()
            .package_entry_index
            .contains_key(&self.pkg_id())
    }
}
