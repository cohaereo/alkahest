use crate::overlays::gui::OverlayProvider;
use crate::packages::package_manager;
use crate::resources::Resources;
use destiny_pkg::TagHash;
use imgui::Ui;
use std::fs::File;
use std::io::Write;
use tracing::error;
use winit::window::Window;

pub struct TagDumper {
    package_id: String,
    entry_id: String,
    tag_string: String,
    message: Result<String, String>,

    use_full_hash: bool,
}

impl TagDumper {
    pub fn new() -> TagDumper {
        TagDumper {
            package_id: String::new(),
            entry_id: String::new(),
            tag_string: String::new(),
            message: Ok(String::new()),
            use_full_hash: true,
        }
    }

    fn dump_entry(&self, tag: TagHash) -> Result<String, String> {
        let entry_header = package_manager().get_entry(tag);
        if let Ok(entry) = entry_header {
            std::fs::create_dir("tags").unwrap();

            let file_path = format!(
                "tags/{tag}_{0}_{1}.bin",
                entry.file_type, entry.file_subtype,
            );
            let mut file = File::create(&file_path).unwrap();
            if file
                .write_all(package_manager().read_tag(tag).unwrap().as_slice())
                .is_err()
            {
                error!("Failed to write resource {file_path} to disk!");
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

impl OverlayProvider for TagDumper {
    fn create_overlay(&mut self, ui: &mut Ui, _window: &Window, _resources: &mut Resources) {
        ui.window("Tag Dumper").build(|| {
            ui.group(|| {
                ui.radio_button("Full hash", &mut self.use_full_hash, true);
                ui.same_line();
                ui.radio_button("Split hash", &mut self.use_full_hash, false);

                let pressed_enter = if self.use_full_hash {
                    ui.input_text("Tag", &mut self.tag_string)
                        .hint("XXXXXXXX")
                        .enter_returns_true(true)
                        .build()
                } else {
                    ui.input_text("Package ID", &mut self.package_id)
                        .hint("XXXX")
                        .build();

                    ui.input_text("Entry Index", &mut self.entry_id)
                        .enter_returns_true(true)
                        .build()
                };

                if ui.button("Dump!") || pressed_enter {
                    if self.use_full_hash {
                        let tag = u32::from_str_radix(&self.tag_string, 16);

                        if let Ok(tag) = tag {
                            self.message = self.dump_entry(TagHash(u32::from_be(tag)));
                        } else {
                            self.message = Err("Malformed input tag.".to_string());
                        }
                    } else {
                        let pkg = u16::from_str_radix(&self.package_id, 16);
                        let entry = self.entry_id.parse();

                        if let (Ok(pkg), Ok(entry)) = (pkg, entry) {
                            self.message = self.dump_entry(TagHash::new(pkg, entry));
                        } else {
                            self.message = Err("Malformed input tag.".to_string());
                        }
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
