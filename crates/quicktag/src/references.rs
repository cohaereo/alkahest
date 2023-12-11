use eframe::epaint::mutex::RwLock;
use log::warn;
use nohash_hasher::IntMap;

use crate::packages::{package_manager, package_manager_checked};

// TODO(cohae): User-defined references
lazy_static::lazy_static! {
    pub static ref REFERENCE_MAP_BASE_PRIMITIVES: IntMap<u32, &'static str> = IntMap::from_iter([
        (0x80800000, "SBungieScript"),
        (0x80800005, "Char"),
        (0x80800009, "Byte"),
        (0x80800090, "Vec4"),
    ]);

    pub static ref REFERENCE_MAP_TTK: IntMap<u32, &'static str> = IntMap::from_iter([
        (0x8080035A, "SLocalizedStrings"),
        (0x808008BE, "SLocalizedStringsData"),
        (0x80801AD0, "SScope"),
        (0x80801B4C, "STechnique"),
    ]);

    pub static ref REFERENCE_MAP_SK: IntMap<u32, &'static str> = IntMap::from_iter([]);

    pub static ref REFERENCE_MAP_BL: IntMap<u32, &'static str> = IntMap::from_iter([
        (0x80800000, "SBungieScript"),
        (0x80808E8E, "SActivity"),
        (0x808045EB, "SMusicTemplate"),
        (0x8080BFE6, "SUnkMusicE6BF8080"),
        (0x8080BFE8, "SUnkMusicE8BF8080"),
        (0x80809AD8, "SEntity"),
        (0x80806F07, "SEntityModel"),
        (0x80806EC5, "SEntityModelMesh"),
        (0x80806695, "CubemapResource"),
        (0x80806DBA, "SScope"),
        (0x808051F2, "SDyeChannels"),
        (0x80804F2C, "SDyeChannelHash"),
        (0x80806DAA, "STechnique"),
        (0x80807211, "STextureTag"),
        (0x80806DCF, "STextureTag64"),
        (0x808093AD, "SStaticMapData"),
        (0x808093B1, "SOcclusionBounds"),
        (0x808093B3, "SMeshInstanceOcclusionBounds"),
        (0x80806D40, "SStaticMeshInstanceTransform"),
        (0x808093BD, "SStaticMeshHash"),
        (0x80806D28, "SStaticMeshInstanceMap"),
        (0x8080891E, "SBubbleParent"),
        (0x80808701, "SBubbleDefinition"),
        (0x80808703, "SMapContainerEntry"),
        (0x80808707, "SMapContainer"),
        (0x80808709, "SMapDataTableEntry"),
        (0x80809883, "SMapDataTable"),
        (0x80809885, "SMapDataEntry"),
        (0x80806CC9, "SMapDataResource"),
        (0x80806A0D, "SStaticMapParent"),
        (0x80806D44, "SStaticMesh"),
        (0x80800014, "SMaterialHash"),
        (0x80806D2F, "SStaticMeshDecal"),
        (0x80806D30, "SStaticMeshData"),
        (0x80806D38, "SStaticMeshMaterialAssignment"),
        (0x80806D37, "SStaticMeshPart"),
        (0x80806D36, "SStaticMeshBuffers"),
        (0x80806C81, "STerrain"),
        (0x80806C86, "SMeshGroup"),
        (0x80806C84, "SStaticPart"),
        (0x808099EF, "SLocalizedStrings"),
        (0x808099F1, "SLocalizedStringsData"),
        (0x808099F7, "SStringPart"),
        (0x808099F5, "SStringPartDefinition"),
        (0x80809B06, "SEntityResource")
    ]);

    pub static ref REFERENCE_NAMES: RwLock<IntMap<u32, &'static str>> = RwLock::new(Default::default());
}

pub fn initialize_reference_names() {
    if package_manager_checked().is_err() {
        panic!("Called initialize_reference_names, but package manager is not initialized!")
    }

    let mut references: IntMap<u32, &'static str> = REFERENCE_MAP_BASE_PRIMITIVES.clone();

    let version_specific = match package_manager().version {
        destiny_pkg::PackageVersion::DestinyTheTakenKing => REFERENCE_MAP_TTK.clone(),
        destiny_pkg::PackageVersion::Destiny2Shadowkeep => REFERENCE_MAP_SK.clone(),
        destiny_pkg::PackageVersion::Destiny2BeyondLight
        | destiny_pkg::PackageVersion::Destiny2WitchQueen
        | destiny_pkg::PackageVersion::Destiny2Lightfall => REFERENCE_MAP_BL.clone(),
        _ => {
            warn!(
                "No reference table found for {:?}",
                package_manager().version
            );
            Default::default()
        }
    };

    references.extend(version_specific);

    *REFERENCE_NAMES.write() = references;
}
