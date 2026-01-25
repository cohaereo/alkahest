use alkahest_data::{
    map::SBubbleParent,
    pattern::SPattern,
    tfx::features::{dynamic::SDynamicModel, statics::SStaticMesh},
};
use egui::{Color32, RichText, TextEdit, Widget};
use google_material_symbols::GoogleMaterialSymbols;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package::UEntryHeader, package_manager};

use super::TabResult;
use crate::ui::util::UiExt;

#[derive(Default)]
pub struct TagLookupTab {
    input: String,
    tag_type: Option<TagType>,
}

impl TagLookupTab {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> TabResult {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                TextEdit::singleline(&mut self.input)
                    .hint_text(RichText::new("80XXXXXX").weak().italics())
                    .ui(ui);

                ui.label(self.validate());
            });
            ui.add_enabled_ui(
                self.tag_type.is_some() && !matches!(self.tag_type, Some(TagType::Unknown(_))),
                |ui| {
                    let _ = ui.d_button("Open");
                },
            );
        });

        TabResult::Continue
    }

    fn validate(&mut self) -> RichText {
        let Ok(tag) = str::parse::<TagHash>(&self.input) else {
            self.tag_type = None;
            return RichText::new("Invalid tag hash")
                .color(Color32::RED)
                .strong();
        };

        let Some(entry) = package_manager().get_entry(tag) else {
            self.tag_type = None;
            return RichText::new("Tag not found").color(Color32::RED).strong();
        };

        let ttype = TagType::from_entry(&entry);
        self.tag_type = Some(ttype);
        match ttype {
            TagType::Map => RichText::new(format!("{} Map", GoogleMaterialSymbols::Map,)),
            TagType::Texture => RichText::new(format!("{} Texture", GoogleMaterialSymbols::Image,)),
            TagType::DynamicModel => RichText::new(format!(
                "{} Dynamic Model",
                GoogleMaterialSymbols::DeployedCode,
            )),
            TagType::StaticModel => {
                RichText::new(format!("{} Static Model", GoogleMaterialSymbols::Landscape,))
            }
            TagType::Pattern => {
                RichText::new(format!("{} Pattern", GoogleMaterialSymbols::Category,))
            }
            TagType::Unknown(u) => RichText::new(format!(
                "{} Unknown tag type {u:08X}",
                GoogleMaterialSymbols::QuestionMark,
            ))
            .color(Color32::GRAY),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TagType {
    Map,
    Texture,
    DynamicModel,
    StaticModel,
    Pattern,
    Unknown(u32),
}

impl TagType {
    pub fn from_entry(entry: &UEntryHeader) -> Self {
        if entry.file_type == 32 && matches!(entry.file_subtype, 1..=3) {
            return Self::Texture;
        }

        match Some(entry.reference) {
            SBubbleParent::ID => Self::Map,
            SDynamicModel::ID => Self::DynamicModel,
            SStaticMesh::ID => Self::StaticModel,
            SPattern::ID => Self::Pattern,
            _ => Self::Unknown(entry.reference),
        }
    }
}
