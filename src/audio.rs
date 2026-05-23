use std::sync::{LazyLock, atomic::AtomicU64};

use ahash::AHashMap;
use alkahest_data::wwise::SWwiseEvent;
use anyhow::Context;
use glam::Vec3;
use parking_lot::Mutex;
use rrise::{
    AkCallbackType, AkSoundPosition, AkVector, AkVector64,
    settings::{
        AkDeviceSettings, AkInitSettings, AkMemSettings, AkMusicSettings, AkPlatformInitSettings,
        AkStreamMgrSettings,
    },
};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

pub const LISTENER_ID: u64 = 1;
static GAMEOBJECT_TRACKER: AtomicU64 = AtomicU64::new(2);
static FREE_GAMEOBJECT_IDS: Mutex<Vec<u64>> = Mutex::new(Vec::new());

static BANK_CACHE: LazyLock<Mutex<AHashMap<TagHash, (rrise::AkBankID, Vec<u8>)>>> =
    LazyLock::new(|| Mutex::new(AHashMap::default()));

pub struct AudioSource {
    pub gameobject_id: u64,
    bank: rrise::AkBankID,
}

impl AudioSource {
    pub fn load_event_and_play(event_tag: TagHash) -> anyhow::Result<Self> {
        let event: SWwiseEvent = package_manager()
            .read_tag_struct(event_tag)
            .context("Failed to read event tag")?;

        if !BANK_CACHE.lock().contains_key(&event.wwise_bank) {
            for stream in event.wwise_streams {
                let Some(entry) = package_manager().get_entry(stream) else {
                    continue;
                };
                let stream_path = format!("Media/{}.wem", entry.reference);
                if !std::path::Path::new(&stream_path).exists() {
                    let stream_data = package_manager()
                        .read_tag(stream)
                        .context("Failed to read stream")?;
                    _ = std::fs::write(format!("Media/{}.wem", entry.reference), stream_data);
                }
            }
        }

        let source = Self::load_bank(event.wwise_bank)?;
        source.post_event(event.event_id)?;
        rrise::sound_engine::set_game_object_output_bus_volume(
            source.gameobject_id,
            LISTENER_ID,
            0.0,
        );
        Ok(source)
    }

    pub fn load_bank(bank_tag: TagHash) -> anyhow::Result<Self> {
        let bank_id = match BANK_CACHE.lock().entry(bank_tag) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.get().0,
            std::collections::hash_map::Entry::Vacant(entry) => {
                let bank_data = package_manager()
                    .read_tag(bank_tag)
                    .context("Failed to read bank")?;
                let bank_id = rrise::sound_engine::load_bank_from_memory(&bank_data)
                    .context("Failed to load bank")?;
                entry.insert((bank_id, bank_data)).0
            }
        };
        let gameobject_id = alloc_gameobject_id();
        rrise::sound_engine::register_game_obj(gameobject_id)?;

        Ok(AudioSource {
            gameobject_id,
            bank: bank_id,
        })
    }

    pub fn set_position(&self, pos: Vec3) {
        set_gameobject_pos(self.gameobject_id, pos, Vec3::Z, Vec3::X, false);
        // set_gameobject_pos(self.gameobject_id, Vec3::ZERO, Vec3::Z, Vec3::X, false);
    }

    pub fn post_event(&self, event_id: u32) -> Result<rrise::AkPlayingID, rrise::AkResult> {
        rrise::sound_engine::PostEvent::new(self.gameobject_id, event_id).post()
    }
}

impl Drop for AudioSource {
    fn drop(&mut self) {
        rrise::sound_engine::unregister_game_obj(self.gameobject_id).ok();
        free_gameobject_id(self.gameobject_id);
    }
}

pub fn alloc_gameobject_id() -> u64 {
    if let Some(id) = FREE_GAMEOBJECT_IDS.lock().pop() {
        id
    } else {
        GAMEOBJECT_TRACKER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

pub fn free_gameobject_id(id: u64) {
    FREE_GAMEOBJECT_IDS.lock().push(id);
}

pub fn set_gameobject_pos(id: u64, pos: Vec3, up: Vec3, front: Vec3, is_listener: bool) {
    if let Err(e) = rrise::sound_engine::set_position(
        id,
        AkSoundPosition {
            position: AkVector64 {
                X: pos.x as f64,
                Y: pos.y as f64,
                Z: pos.z as f64,
            },
            orientationFront: AkVector {
                X: front.x,
                Y: front.y,
                Z: front.z,
            },
            orientationTop: AkVector {
                X: up.x,
                Y: up.y,
                Z: up.z,
            },
        },
        is_listener,
    ) {
        error!("Failed to set game object position: {}", e);
    }
}

pub fn init_sound_engine() -> anyhow::Result<()> {
    // init memorymgr
    rrise::memory_mgr::init(&mut AkMemSettings::default())?;
    assert!(rrise::memory_mgr::is_initialized());

    // init streamingmgr
    rrise::stream_mgr::init_default_stream_mgr(
        &AkStreamMgrSettings::default(),
        &mut AkDeviceSettings::default(),
        "./",
    )?;
    rrise::stream_mgr::set_current_language("English(US)")?;

    let plugin_dir_path = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("plugins")
        .to_path_buf();

    // init soundengine
    rrise::sound_engine::init(
        &mut AkInitSettings::default().with_plugin_dll_path(plugin_dir_path.display().to_string()),
        &mut AkPlatformInitSettings::default(),
    )?;

    rrise::spatial_audio::init()?;

    rrise::music_engine::init(&mut AkMusicSettings::default())?;

    info!("Loading init bank");
    let (init_bank_tag, _) = package_manager()
        .get_all_by_type(26, Some(5))
        .first()
        .cloned()
        .context("No init bank found")?;

    let init_bank_data = package_manager()
        .read_tag(init_bank_tag)
        .context("Failed to read init bank")?;
    rrise::sound_engine::load_bank_from_memory(&init_bank_data)
        .context("Failed to load init bank")?;

    info!("Sound engine initialized");

    #[cfg(debug_assertions)]
    rrise::communication::init(&rrise::settings::AkCommSettings::default())?;

    rrise::sound_engine::register_game_obj(LISTENER_ID)?;
    rrise::sound_engine::set_default_listeners(&[LISTENER_ID])?;
    // rrise::sound_engine::set_listener_spatialization(LISTENER_ID, true)?;

    Ok(())
}

pub fn term_sound_engine() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    rrise::communication::term();

    rrise::sound_engine::stop_all(None);
    rrise::sound_engine::unregister_all_game_obj()?;

    // term music
    rrise::music_engine::term();

    // term soundengine
    rrise::sound_engine::term();

    // term streamingmgr
    rrise::stream_mgr::term_default_stream_mgr();

    // term memorymgr
    rrise::memory_mgr::term();

    Ok(())
}
