use egui::{ImageSource, include_image};

macro_rules! include_icon {
    ($name:ident, $filename:expr) => {
        pub const $name: ImageSource = include_image!(concat!("../../assets/ui/icons/", $filename));
    };
}

include_icon!(CRUCIBLE, "crucible.svg");
include_icon!(DUNGEON, "dungeon.svg");
include_icon!(ENGRAM, "engram.svg");
include_icon!(GAMBIT, "gambit.svg");
include_icon!(IRON_BANNER, "iron-banner.svg");
include_icon!(LOST_SECTOR, "lost-sector.svg");
include_icon!(OSIRIS, "osiris.svg");
include_icon!(PATROL, "patrol.svg");
include_icon!(QUEST, "quest.svg");
include_icon!(RAID, "raid.svg");
include_icon!(STRIKE, "strike.svg");
include_icon!(UNKNOWN, "unknown.svg");
