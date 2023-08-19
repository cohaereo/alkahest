use std::fs::File;
use std::io::Write;
use destiny_pkg::TagHash;
use imgui::Ui;
use winit::window::Window;
use crate::overlays::gui::OverlayProvider;
use crate::packages::package_manager;
use crate::resources::Resources;


pub struct PackageDumper {
    package_id: String,
    entry_id: String,
    message: Result<String, String>
}

impl PackageDumper {

    pub fn new() -> PackageDumper {
        return PackageDumper{ package_id: "".to_string(), entry_id: "".to_string(), message: Ok("".to_string()) };
    }

    fn dump_entry(&self, pkg_id: u16, entry_id: u16) -> Result<String, String> {
        let tag = TagHash::new(pkg_id, entry_id);
        let entry_header = package_manager().get_entry(tag);
        if entry_header.is_ok() {
            let entry = entry_header.unwrap();
            let mut file = File::create(format!("{tag}.{0}.{1}.tag", entry.file_subtype, entry.file_type)).unwrap();
            if file.write_all(package_manager().read_tag(tag).unwrap().as_slice()).is_err() {
                println!("Failed to write resource {0} to disk!", format!("{tag}.{0}.{1}.tag", entry.file_subtype, entry.file_type));
                return Err("Failed to dump tag!".to_string());
            } else {
                return Ok("Dumped!".to_string());
            }
        } else {
            println!("Unable to find resource {0}!", tag);
            return Err("Failed to dump tag!".to_string());

        }
    }
}

impl OverlayProvider for PackageDumper {
    fn create_overlay(&mut self, ui: &mut Ui, window: &Window, resources: &mut Resources) {
        ui.window("Extractor").build(||{
                ui.group(|| {
                    ui.input_text("Package ID", &mut self.package_id).hint("XXXX").enter_returns_true(false).build();
                    ui.input_text("Entry ID", &mut self.entry_id).hint("XXXX").enter_returns_true(false).build();
                    if ui.button("Dump!") {
                        let pkg = u16::from_str_radix(&self.package_id, 16);
                        let entry = u16::from_str_radix(&self.entry_id, 10);
                        if pkg.is_err() || entry.is_err() {
                            self.message = Err("Malformed input tag.".to_string());
                        } else {
                            self.message = self.dump_entry(pkg.unwrap(), entry.unwrap());
                        }
                    }

                    match self.message.as_ref() {
                        Ok(msg) => ui.text_colored([0.0, 1.0, 0.0, 1.0], msg),
                        Err(msg) => ui.text_colored([1.0, 0.0, 0.0, 1.0], msg)
                    }
                });
            });
    }
}