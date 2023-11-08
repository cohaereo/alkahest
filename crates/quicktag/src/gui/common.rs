use destiny_pkg::{TagHash, TagHash64};
use eframe::egui;
use log::warn;

use crate::packages::package_manager;

pub fn tag_context(ui: &mut egui::Ui, tag: TagHash, tag64: Option<TagHash64>) {
    if ui.selectable_label(false, "📋 Copy tag").clicked() {
        ui.output_mut(|o| o.copied_text = tag.to_string());
        ui.close_menu();
    }

    if let Some(tag64) = tag64 {
        if ui.selectable_label(false, "📋 Copy 64-bit tag").clicked() {
            ui.output_mut(|o| o.copied_text = tag64.to_string());
            ui.close_menu();
        }
    }

    if ui
        .add_enabled(
            false,
            egui::SelectableLabel::new(false, "📤 Open in Alkahest"),
        )
        .clicked()
    {
        warn!("Alkahest IPC not implemented yet");
        ui.close_menu();
    }
}

pub fn open_tag_in_default_application(tag: TagHash) {
    let data = package_manager().read_tag(tag).unwrap();
    let entry = package_manager().get_entry(tag).unwrap();

    let filename = format!(
        "{tag}_ref-{:08X}_{}_{}.bin",
        entry.reference.to_be(),
        entry.file_type,
        entry.file_subtype,
    );

    let path = std::env::temp_dir().join(filename);
    std::fs::write(&path, data).ok();

    opener::open(path).ok();
}
