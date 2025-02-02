use std::{borrow::Cow, collections::HashMap, sync::Arc};

use alkahest_data::entity::SEntity;
use alkahest_pm::{package_manager, PACKAGE_MANAGER};
use destiny_pkg::{GameVersion, PackageManager, TagHash};
use lazy_static::lazy_static;
use tiger_parse::PackageManagerExt;

lazy_static! {
    static ref TYPE_MAPPING: HashMap<u32, &'static str> = [
        // Unk8 types
        // (0x80809AF3, "CommonComponent"),

        // No-brainer components
        (0x80806D8A, "EntityModel"),
        (0x80806D5B, "PhysicsModel"),
        (0x808081DD, "SkeletonFK"),
        (0x80808B66, "SkeletonIK"),
        (0x80806C5D, "ExpensiveLight"),
        (0x808067B3, "LensFlare"),
        (0x8080410A, "PlayerCollisionSound"),
        (0x8080916A, "HavokCollider"),
        (0x80809479, "Sequence"),

        (0x80803058, "AscendantPlatform"),

        // Components I'm not entirely sure about
        (0x80802613, "UnkAnimationClips1"),
        (0x80802641, "UnkAnimationClips2"),
        (0x808028B1, "UnkAnimationClips3"),
        (0x80805EDA, "UnkDialogWithAnimation"),
        (0x80806629, "UnkExpressions80806629"),
        (0x80809779, "UnkMaybeAttachmentPoints80809779"),
        (0x80E13581, "UnkScripts80E13581"),
        (0x80802E08, "UnkExpressionsAttributes80802E08"),
        (0x80802BF4, "UnkCameraRelated80802BF4"),
        (0x8080478E, "UnkInteractible8080478E"),
        (0x80802B21, "UnkInteractible80802B21"),
        (0x80802B1B, "UnkInteractible80802B1B"),
        (0x80805FAA, "UnkMaybeCutscenePlayer80805FAA"),

        // Components without a clear purpose / empty components
        (0x80808158, "Unk80808158"),
        (0x80802B8E, "Unk80802B8E"),
        (0x808039BC, "Unk808039BC"),
        (0x80802503, "Unk80802503"),
        (0x80802896, "Unk80802896"),
        (0x80802C58, "Unk80802C58"),
        (0x80808EDB, "Unk80808EDB"),
        (0x80802DA5, "Unk80802DA5"),
        (0x80803795, "Unk80803795"),
        (0x808037D1, "Unk808037D1"),
        (0x808032B1, "Unk808032B1"),
        (0x808060A5, "Unk808060A5"),
        (0x80809219, "Unk80809219"),
        (0x80802383, "Unk80802383"),
        (0x808062D0, "Unk808062D0"),
        (0x808095B1, "Unk808095B1"),
        (0x80802832, "Unk80802832"),
    ]
    .into_iter()
    .collect();
}

fn get_type_name(tag: u32) -> Cow<'static, str> {
    TYPE_MAPPING
        .get(&tag)
        .copied()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Owned(format!("{:08X}", tag)))
}

fn main() -> anyhow::Result<()> {
    let packages_dir = std::env::args().nth(1).expect("missing packages dir");
    *PACKAGE_MANAGER.write() = Some(Arc::new(PackageManager::new(
        packages_dir,
        GameVersion::Destiny2TheFinalShape,
        None,
    )?));
    println!("Package manager initialized");

    let specific_tag = std::env::args()
        .nth(2)
        .map(|s| TagHash(u32::from_be(u32::from_str_radix(&s, 16).unwrap())));

    for (tag, _) in package_manager().get_all_by_reference(0x80809AD8) {
        if let Some(specific_tag) = specific_tag {
            if tag != specific_tag {
                continue;
            }
        }

        let ent: SEntity = package_manager().read_tag_struct(tag)?;
        println!("ent {tag}");
        for res in ent.entity_resources {
            println!(
                "  component {}: {} / {} / {:08X}",
                res.unk0.taghash(),
                get_type_name(res.unk0.unk8.resource_type),
                get_type_name(res.unk0.unk10.resource_type),
                res.unk0.unk18.resource_type
            );
        }
        println!();
    }

    Ok(())
}
