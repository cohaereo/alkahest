use std::io::Write;

use alkahest_data::ExtendedHash;
use destiny_pkg::{package::UEntryHeader, TagHash};
use fs_err::File;
use tracing::error;
use winit::window::Window;

use crate::{
    overlays::gui::Overlay, packages::package_manager, resources::Resources, texture::Texture,
    util::dds,
};

pub struct TagDumper {
    package_id: String,
    entry_index: String,
    tag_string: String,
    message: Option<Result<(TagHash, UEntryHeader), String>>,

    use_full_hash: bool,
}

impl TagDumper {
    pub fn new() -> TagDumper {
        TagDumper {
            package_id: String::new(),
            entry_index: String::new(),
            tag_string: String::new(),
            message: None,
            use_full_hash: true,
        }
    }

    fn dump_entry(&self, tag: TagHash) -> Result<UEntryHeader, String> {
        let entry_header = package_manager().get_entry(tag);
        if let Some(entry) = entry_header {
            std::fs::create_dir("tags").ok();

            let file_path = format!(
                "tags/{tag}_ref-{:08X}_{}_{}.bin",
                entry.reference.to_be(),
                entry.file_type,
                entry.file_subtype,
            );
            let mut file = File::create(&file_path).unwrap();
            if let Err(e) = file.write_all(package_manager().read_tag(tag).unwrap().as_slice()) {
                error!("Failed to write tag {file_path} to disk: {e}");
                Err(format!("Failed to dump tag!\n{e}"))
            } else {
                Ok(entry)
            }
        } else {
            error!("Unable to find tag {tag}!");
            Err("Failed to dump tag!".to_string())
        }
    }
}

impl Overlay for TagDumper {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        _resources: &mut Resources,
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        egui::Window::new("Tag Dumper").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.use_full_hash, true, "Full hash");
                ui.radio_value(&mut self.use_full_hash, false, "Split hash");
            });

            let pressed_enter = if self.use_full_hash {
                ui.label("Tag");
                ui.text_edit_singleline(&mut self.tag_string).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
            } else {
                ui.label("Package ID");
                ui.text_edit_singleline(&mut self.package_id);

                ui.label("Entry Index");
                ui.text_edit_singleline(&mut self.entry_index).lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
            };

            if ui.button("Dump!").clicked() || pressed_enter {
                if self.use_full_hash {
                    let tag = u32::from_str_radix(&self.tag_string, 16);

                    if let Ok(tag) = tag {
                        let tag = TagHash(u32::from_be(tag));
                        self.message = Some(self.dump_entry(tag).map(|v| (tag, v)));
                    } else {
                        self.message = Some(Err("Malformed input tag.".to_string()));
                    }
                } else {
                    let pkg = u16::from_str_radix(&self.package_id, 16);
                    let entry = self.entry_index.parse();

                    if let (Ok(pkg), Ok(entry)) = (pkg, entry) {
                        let tag = TagHash::new(pkg, entry);
                        self.message = Some(self.dump_entry(tag).map(|v| (tag, v)));
                    } else {
                        self.message = Some(Err("Malformed input tag.".to_string()));
                    }
                }
            }

            if let Some(result) = self.message.as_ref() {
                match result {
                    Ok((tag, entry)) => {
                        let msg = format!(
                            "Dumped {tag} / {:04X}_{:04X}\nReference: {:08X}\nType: {}, Subtype: {}",
                            tag.pkg_id(),
                            tag.entry_index(),
                            entry.reference.to_be(),
                            entry.file_type,
                            entry.file_subtype
                        );
                        ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN))
                    }
                    Err(msg) => ui.label(egui::RichText::new(msg).color(egui::Color32::RED)),
                };
            }
        });

        true
    }
}

#[derive(Default)]
pub struct BulkTextureDumper {
    input: String,
    bake_png: bool,
}

impl Overlay for BulkTextureDumper {
    fn draw(
        &mut self,
        ctx: &egui::Context,
        _window: &Window,
        _resources: &mut Resources,
        _gui: &mut super::gui::GuiContext<'_>,
    ) -> bool {
        egui::Window::new("Bulk Texture Dumper").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.bake_png, false, "DDS");
                ui.set_enabled(false);
                ui.radio_value(&mut self.bake_png, true, "PNG");
            });

            if ui.button("Dump!").clicked() {
                for i in self.input.lines() {
                    if let Ok(itag) = u32::from_str_radix(i, 16) {
                        let tag = TagHash(itag.to_be());
                        let mut dds_data: Vec<u8> = vec![];
                        let (texture, texture_data) =
                            Texture::load_data(ExtendedHash::Hash32(tag), true).unwrap();

                        dds::dump_to_dds(&mut dds_data, &texture, &texture_data);
                        std::fs::create_dir("./textures/").ok();
                        if let Ok(mut f) = File::create(format!("./textures/{}.dds", tag)) {
                            f.write_all(&dds_data).ok();
                        }
                    }
                }
            }

            ui.text_edit_multiline(&mut self.input);
        });

        true
    }
}
