use crate::overlays::gui::OverlayProvider;
use crate::packages::package_manager;
use crate::resources::Resources;
use destiny_pkg::TagHash;
use imgui::Ui;
use std::fs::File;
use std::io::Write;
use tracing::error;
use winit::window::Window;

pub struct PackageDumper {
    package_id: String,
    entry_id: String,
    message: Result<String, String>,
}

impl PackageDumper {
    pub fn new() -> PackageDumper {
        PackageDumper {
            package_id: "".to_string(),
            entry_id: "".to_string(),
            message: Ok("".to_string()),
        }
    }

    fn dump_entry(&self, pkg_id: u16, entry_id: u16) -> Result<String, String> {
        let tag = TagHash::new(pkg_id, entry_id);
        let entry_header = package_manager().get_entry(tag);
        if let Ok(entry) = entry_header {
            let mut file = File::create(format!(
                "{tag}.{0}.{1}.tag",
                entry.file_subtype, entry.file_type
            ))
            .unwrap();
            if file
                .write_all(package_manager().read_tag(tag).unwrap().as_slice())
                .is_err()
            {
                error!(
                    "Failed to write resource {} to disk!",
                    format!("{tag}.{0}.{1}.tag", entry.file_subtype, entry.file_type)
                );
                Err("Failed to dump tag!".to_string())
            } else {
                Ok("Dumped!".to_string())
            }
        } else {
            error!("Unable to find resource {tag}!");
            Err("Failed to dump tag!".to_string())
        }
    }
}

impl OverlayProvider for PackageDumper {
    fn create_overlay(&mut self, ui: &mut Ui, _window: &Window, _resources: &mut Resources) {
        ui.window("Extractor").build(|| {
            ui.group(|| {
                ui.input_text("Package ID", &mut self.package_id)
                    .hint("XXXX")
                    .enter_returns_true(false)
                    .build();
                ui.input_text("Entry Index", &mut self.entry_id)
                    .enter_returns_true(false)
                    .build();

                if ui.button("Dump!") {
                    let pkg = u16::from_str_radix(&self.package_id, 16);
                    let entry = self.entry_id.parse();
                    if let (Ok(pkg), Ok(entry)) = (pkg, entry) {
                        self.message = self.dump_entry(pkg, entry);
                    } else {
                        self.message = Err("Malformed input tag.".to_string());
                    }
                }

                match self.message.as_ref() {
                    Ok(msg) => ui.text_colored([0.0, 1.0, 0.0, 1.0], msg),
                    Err(msg) => ui.text_colored([1.0, 0.0, 0.0, 1.0], msg),
                }
            });
        });
    }
}
