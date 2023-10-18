use destiny_pkg::{TagHash, TagHash64};
use eframe::egui;
use log::warn;

use crate::packages::package_manager;

pub fn tag_context(ui: &mut egui::Ui, tag: TagHash) {
    if ui.selectable_label(false, "📋 Copy tag").clicked() {
        ui.output_mut(|o| o.copied_text = tag.to_string());
        ui.close_menu();
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

pub fn tag_context64(ui: &mut egui::Ui, tag: TagHash64) {
    if let Some(tag) = package_manager().hash64_table.get(&tag.0) {
        tag_context(ui, tag.hash32)
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
