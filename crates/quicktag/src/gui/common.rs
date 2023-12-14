use std::fs::File;

use destiny_pkg::{TagHash, TagHash64};
use eframe::egui;
use log::{error, warn};

use crate::{packages::package_manager, tagtypes::TagType};

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

    if let Some(entry) = package_manager().get_entry(tag) {
        let shift = ui.input(|i| i.modifiers.shift);

        if ui
            .selectable_label(
                false,
                format!(
                    "📋 Copy reference tag{}",
                    if shift { " (flipped)" } else { "" }
                ),
            )
            .clicked()
        {
            ui.output_mut(|o| {
                o.copied_text = format!(
                    "{:08X}",
                    if shift {
                        entry.reference.to_be()
                    } else {
                        entry.reference
                    }
                )
            });
            ui.close_menu();
        }

        let tt = TagType::from_type_subtype(entry.file_type, entry.file_subtype);
        if tt == TagType::WwiseStream && ui.selectable_label(false, "🎵 Play audio").clicked() {
            open_audio_file_in_default_application(tag, "wem");
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

pub fn open_audio_file_in_default_application(tag: TagHash, ext: &str) {
    let data = package_manager().read_tag(tag).unwrap();

    let filename = format!(".\\{tag}.{ext}");

    let (samples, desc) =
        match vgmstream::read_file_to_samples_no_questions_asked(&data, Some(filename)) {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to decode audio file: {e}");
                return;
            }
        };

    let filename_wav = format!("{tag}.wav");

    let path = std::env::temp_dir().join(filename_wav);
    // std::fs::write(&path, data).ok();
    if let Ok(mut f) = File::create(&path) {
        wav::write(
            wav::Header {
                audio_format: wav::WAV_FORMAT_PCM,
                channel_count: desc.channels as u16,
                sampling_rate: desc.sample_rate as u32,
                bytes_per_second: desc.bitrate as u32,
                bytes_per_sample: 2,
                bits_per_sample: 16,
            },
            &wav::BitDepth::Sixteen(samples),
            &mut f,
        )
        .unwrap();

        opener::open(path).ok();
    }
}
