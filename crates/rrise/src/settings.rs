/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use std::sync::atomic::{AtomicPtr, Ordering};

use log::error;

#[cfg(not(wwrelease))]
pub use crate::bindings::root::AkCommSettings;
#[cfg(not(wwrelease))]
use crate::bindings::root::AK::Comm;
pub use crate::bindings::root::{AkMemSettings, AkMusicSettings, AkStreamMgrSettings};
use crate::{
    bindings::root::{
        AkJobMgrSettings,
        AK::{MemoryMgr, MusicEngine, SoundEngine, StreamMgr},
    },
    to_os_char, OsChar,
};

impl Default for AkMemSettings {
    /// Obtain the default initialization settings for the default implementation of the Memory Manager.
    fn default() -> Self {
        unsafe {
            let mut ss: AkMemSettings = std::mem::zeroed();
            MemoryMgr::GetDefaultSettings(&mut ss);
            ss
        }
    }
}

impl Default for AkMusicSettings {
    /// Get the default values of the initialization settings of the music engine.
    fn default() -> Self {
        unsafe {
            let mut ss: AkMusicSettings = std::mem::zeroed();
            MusicEngine::GetDefaultInitSettings(&mut ss);
            ss
        }
    }
}

impl Default for AkStreamMgrSettings {
    /// Get the default values for the Stream Manager's settings.
    ///
    /// *See also*
    /// > - [stream_mgr::init](crate::stream_mgr::init)
    /// > - [stream_mgr::init_default_stream_mgr](crate::stream_mgr::init_default_stream_mgr)
    fn default() -> Self {
        unsafe {
            let mut ss: AkStreamMgrSettings = std::mem::zeroed();
            StreamMgr::GetDefaultSettings(&mut ss);
            ss
        }
    }
}

#[cfg(not(wwrelease))]
impl Default for AkCommSettings {
    /// Gets the communication module's default initialization settings values.
    ///
    /// *See also*
    /// > - [communication::init](crate::communication::init)
    fn default() -> Self {
        unsafe {
            let mut ss: AkCommSettings = std::mem::zeroed();
            Comm::GetDefaultInitSettings(&mut ss);
            if let Some(app_name) = app_name() {
                ss.szAppNetworkName = app_name;
            }
            ss
        }
    }
}

#[cfg(not(wwrelease))]
unsafe fn app_name() -> Option<[i8; 64]> {
    if let Some(mut name) = std::env::current_exe()
        .ok()?
        .file_name()?
        .to_str()?
        .to_owned()
        .into()
    {
        if name.ends_with(".exe") {
            name.truncate(name.len() - 4);
        }

        if name.len() < 64 {
            name.extend(std::iter::repeat('\0').take(64 - name.len()));
        } else {
            name.truncate(64);
        }

        let mut truncated = [0_u8; 64];
        truncated.copy_from_slice(name.as_bytes());
        truncated[63] = 0;

        Some(std::mem::transmute(truncated))
    } else {
        None
    }
}

pub struct AkInitSettingsPrivate {
    #[doc = "When using DLLs for plugins, specify their path. Leave NULL if DLLs are in the same \
             folder as the game executable."]
    plugin_dll_path: Vec<OsChar>,
}

impl Default for AkInitSettingsPrivate {
    fn default() -> Self {
        Self {
            plugin_dll_path: vec![0],
        }
    }
}

/// Platform-independent initialization settings of the sound engine
/// See also
/// > - [sound_engine::init](crate::sound_engine::init)
/// > - [AkPlatformInitSettings::default]
pub struct AkInitSettings {
    pub install_assert_hook: bool,
    #[doc = "Maximum number of paths for positioning"]
    pub max_num_paths: crate::bindings::root::AkUInt32,
    #[doc = "Size of the command queue, in bytes"]
    pub command_queue_size: crate::bindings::root::AkUInt32,
    #[doc = "Sets to true to enable AK::SoundEngine::PrepareGameSync usage."]
    pub enable_game_sync_preparation: bool,
    #[doc = "Number of quanta ahead when continuous containers should instantiate a new voice \
             before which next sounds should start playing. This look-ahead time allows I/O to \
             occur, and is especially useful to reduce the latency of continuous containers with \
             trigger rate or sample-accurate transitions."]
    #[doc = "Default is 1 audio quantum, also known as an audio frame. Its size is equal to \
             AkInitSettings::num_samples_per_frame / AkPlatformInitSettings::sample_rate. For \
             many platforms the default values - which can be overridden - are respectively 1,024 \
             samples and 48 kHz. This gives a default 21.3 ms for an audio quantum, which is \
             adequate if you have a RAM-based streaming device that completes transfers within 20 \
             ms. With 1 look-ahead quantum, voices spawned by continuous containers are more \
             likely to be ready when they are required to play, thereby improving the overall \
             precision of sound scheduling. If your device completes transfers in 30 ms instead, \
             you might consider increasing this value to 2 because it will grant new voices 2 \
             audio quanta (~43 ms) to fetch data."]
    pub continuous_playback_look_ahead: crate::bindings::root::AkUInt32,
    #[doc = "Number of samples per audio frame (256, 512, 1024, or 2048)."]
    pub num_samples_per_frame: crate::bindings::root::AkUInt32,
    #[doc = "Size of the monitoring queue, in bytes. This parameter is not used in Release build."]
    pub monitor_queue_pool_size: crate::bindings::root::AkUInt32,
    #[doc = "Main output device settings."]
    pub settings_main_output: crate::bindings::root::AkOutputSettings,
    #[doc = "Amount of time to wait for HW devices to trigger an audio interrupt. If there is no \
             interrupt after that time, the sound engine will revert to  silent mode and continue \
             operating until the HW finally comes back. Default value: 2000 (2 seconds)"]
    pub max_hardware_timeout_ms: crate::bindings::root::AkUInt32,
    #[doc = "Use a separate thread for loading sound banks. Allows asynchronous operations."]
    pub use_sound_bank_mgr_thread: bool,
    #[doc = "Use a separate thread for processing audio. If set to false, audio processing will \
             occur in RenderAudio(). \\ref goingfurther_eventmgrthread"]
    pub use_lengine_thread: bool,
    #[doc = "Application-defined audio source change event callback function."]
    pub bgm_callback: crate::bindings::root::AkBackgroundMusicChangeCallbackFunc,
    #[doc = "Application-defined user data for the audio source change event callback function."]
    pub bgm_callback_cookie: AtomicPtr<std::os::raw::c_void>,
    #[doc = "Floor plane axis for 3D game object viewing."]
    pub floor_plane: crate::bindings::root::AkFloorPlane,
    #[doc = "The number of game units in a meter."]
    pub game_units_to_meters: crate::bindings::root::AkReal32,
    // #[doc = "The defined client task scheduler that AkSoundEngine will use to schedule internal tasks."]
    // pub task_scheduler_desc: crate::bindings::root::AkTaskSchedulerDesc,
    #[doc = "The number of bytes read by the BankReader when new data needs to be loaded from \
             disk during serialization. Increasing this trades memory usage for larger, but \
             fewer, file-read events during bank loading."]
    pub bank_read_buffer_size: crate::bindings::root::AkUInt32,
    #[doc = "Debug setting: Only used when debug_out_of_range_check_enabled is true.  This \
             defines the maximum values samples can have.  Normal audio must be contained within \
             +1/-1.  This limit should be set higher to allow temporary or short excursions out \
             of range.  Default is 16."]
    pub debug_out_of_range_limit: crate::bindings::root::AkReal32,
    #[doc = "Debug setting: Enable checks for out-of-range (and NAN) floats in the processing \
             code.  This incurs a small performance hit, but can be enabled in most scenarios.  \
             Will print error messages in the log if invalid values are found at various point in \
             the pipeline. Contact AK Support with the new error messages for more information."]
    pub debug_out_of_range_check_enabled: bool,
    #[doc = "Private information"]
    pub private_stuff: AkInitSettingsPrivate,
    pub settings_job_manager: AkJobMgrSettings,
}

#[derive(Debug)]
/// High-level IO devices initialization settings.
pub struct AkDeviceSettings {
    #[doc = "Pointer for I/O memory allocated by user."]
    #[doc = "Pass NULL if you want memory to be allocated via AK::MemoryMgr::Malign()."]
    #[doc = "If specified, io_memory_size, io_memory_alignment and pool_attributes are ignored."]
    pub io_memory: AtomicPtr<std::os::raw::c_void>,
    #[doc = "Size of memory for I/O (for automatic streams). It is passed directly to \
             AK::MemoryMgr::Malign(), after having been rounded down to a multiple of granularity."]
    pub io_memory_size: u32,
    #[doc = "I/O memory alignment. It is passed directly to AK::MemoryMgr::Malign()."]
    pub io_memory_alignment: u32,
    #[doc = "Attributes for I/O memory. Here, specify the allocation type (AkMemType_Device, and \
             so on). It is passed directly to AK::MemoryMgr::Malign()."]
    pub pool_attributes: u32,
    #[doc = "I/O requests granularity (typical bytes/request)."]
    pub granularity: u32,
    // #[doc = "Scheduler type flags."]
    // pub scheduler_type_flags: u32,
    #[doc = "Scheduler thread properties."]
    pub thread_properties: crate::AkThreadProperties,
    #[doc = "Targeted automatic stream buffer length (ms). When a stream reaches that buffering, \
             it stops being scheduled for I/O except if the scheduler is idle."]
    pub target_auto_stm_buffer_length: f32,
    #[doc = "Maximum number of transfers that can be sent simultaneously to the Low-Level I/O \
             (applies to AK_SCHEDULER_DEFERRED_LINED_UP device only)."]
    pub max_concurrent_io: u32,
    #[doc = "If true the device attempts to reuse IO buffers that have already been streamed from \
             disk. This is particularly useful when streaming small looping sounds. The drawback \
             is a small CPU hit when allocating memory, and a slightly larger memory footprint in \
             the StreamManager pool."]
    pub use_stream_cache: bool,
    #[doc = "Maximum number of bytes that can be \"pinned\" using \
             AK::SoundEngine::PinEventInStreamCache() or AK::IAkStreamMgr::PinFileInCache()"]
    pub max_cache_pinned_bytes: u32,
}

#[derive(Debug)]
pub struct AkPlatformInitSettings {
    #[cfg(windows)]
    #[doc = "Handle to the window associated to the audio."]
    #[doc = "Each game must specify the HWND of the application for device detection purposes."]
    #[doc = "The value returned by GetDefaultPlatformInitSettings is the foreground HWND at"]
    #[doc = "the moment of the initialization of the sound engine and may not be the correct one \
             for your game."]
    #[doc = "It is required that each game provides the correct HWND to be used."]
    pub h_wnd: AtomicPtr<core::ffi::c_void>,
    #[doc = "Lower engine threading properties"]
    pub thread_lengine: crate::AkThreadProperties,
    #[doc = "Ouput thread threading properties"]
    pub thread_output_mgr: crate::AkThreadProperties,
    #[doc = "Bank manager threading properties (its default priority is AK_THREAD_PRIORITY_NORMAL)"]
    pub thread_bank_manager: crate::AkThreadProperties,
    #[doc = "Monitor threading properties (its default priority is \
             AK_THREAD_PRIORITY_ABOVENORMAL). This parameter is not used in Release build."]
    pub thread_monitor: crate::AkThreadProperties,
    #[doc = "Number of refill buffers in voice buffer. 2 == double-buffered, defaults to 4."]
    pub num_refills_in_voice: u16,
    #[doc = "Sampling Rate. Default is 48000 Hz. Use 24000hz for low quality. Any positive \
             reasonable sample rate is supported. However be careful setting a custom value. \
             Using an odd or really low sample rate may result in malfunctionning sound engine."]
    pub sample_rate: u32,
    #[cfg(windows)]
    #[doc = "Enables run-time detection of AVX and AVX2 SIMD support in the engine and plug-ins. \
             Disabling this may improve CPU performance by allowing for higher CPU clockspeeds."]
    pub enable_avx_support: bool,
    #[cfg(windows)]
    #[doc = "Dictates how many Microsoft Spatial Sound dynamic objects will be reserved by the \
             System sink. On Windows, other running processes will be prevented from reserving \
             these objects. Set to 0 to disable the use of System Audio Objects. Default is 128."]
    pub max_system_audio_objects: u32,
    #[cfg(target_os = "linux")]
    #[doc = "Main audio API to use. Leave to \
             [AkAPI_Default](crate::bindings::root::AkAudioAPILinux::AkAPI_Default) for the \
             default sink (default value)."]
    #[doc = "If a valid audioDeviceShareset plug-in is provided, the AkAudioAPI will be Ignored."]
    pub audio_api: crate::bindings::root::AkAudioAPI,
    #[cfg(target_os = "linux")]
    #[doc = "Sample type. [AK_FLOAT](crate::bindings::root::AK_FLOAT) for 32 bit float, \
             [AK_INT](crate::bindings::root::AK_INT) for 16 bit signed integer, defaults to \
             [AK_FLOAT](crate::bindings::root::AK_FLOAT)."]
    #[doc = "Supported by \
             [AkAPI_PulseAudio](crate::bindings::root::AkAudioAPILinux::AkAPI_PulseAudio) only."]
    pub sample_type: u16,
}

impl Default for AkInitSettings {
    /// Gets the default values of the platform-independent initialization settings.
    fn default() -> Self {
        let inner_settings = unsafe {
            let mut ss: crate::bindings::root::AkInitSettings = std::mem::zeroed();
            SoundEngine::GetDefaultInitSettings(&mut ss);
            ss
        };
        Self {
            install_assert_hook: false,
            max_num_paths: inner_settings.uMaxNumPaths,
            command_queue_size: inner_settings.uCommandQueueSize,
            enable_game_sync_preparation: inner_settings.bEnableGameSyncPreparation,
            continuous_playback_look_ahead: inner_settings.uContinuousPlaybackLookAhead,
            num_samples_per_frame: inner_settings.uNumSamplesPerFrame,
            monitor_queue_pool_size: inner_settings.uMonitorQueuePoolSize,
            settings_main_output: inner_settings.settingsMainOutput,
            max_hardware_timeout_ms: inner_settings.uMaxHardwareTimeoutMs,
            use_sound_bank_mgr_thread: inner_settings.bUseSoundBankMgrThread,
            use_lengine_thread: inner_settings.bUseLEngineThread,
            bgm_callback: inner_settings.BGMCallback,
            bgm_callback_cookie: inner_settings.BGMCallbackCookie.into(),
            floor_plane: inner_settings.eFloorPlane,
            game_units_to_meters: inner_settings.fGameUnitsToMeters,
            // task_scheduler_desc: inner_settings.taskSchedulerDesc,
            settings_job_manager: inner_settings.settingsJobManager,
            bank_read_buffer_size: inner_settings.uBankReadBufferSize,
            debug_out_of_range_limit: inner_settings.fDebugOutOfRangeLimit,
            debug_out_of_range_check_enabled: inner_settings.bDebugOutOfRangeCheckEnabled,
            private_stuff: AkInitSettingsPrivate::default(),
        }
    }
}

impl Default for AkDeviceSettings {
    /// Gets the default values of the platform-independent initialization settings.
    fn default() -> Self {
        let inner_settings = unsafe {
            let mut ss: crate::bindings::root::AkDeviceSettings = std::mem::zeroed();
            StreamMgr::GetDefaultDeviceSettings(&mut ss);
            ss
        };
        Self {
            io_memory: inner_settings.pIOMemory.into(),
            io_memory_size: inner_settings.uIOMemorySize,
            io_memory_alignment: inner_settings.uIOMemoryAlignment,
            pool_attributes: inner_settings.ePoolAttributes,
            granularity: inner_settings.uGranularity,
            thread_properties: inner_settings.threadProperties,
            target_auto_stm_buffer_length: inner_settings.fTargetAutoStmBufferLength,
            max_concurrent_io: inner_settings.uMaxConcurrentIO,
            use_stream_cache: inner_settings.bUseStreamCache,
            max_cache_pinned_bytes: inner_settings.uMaxCachePinnedBytes,
        }
    }
}

impl Default for AkPlatformInitSettings {
    /// Gets the default values of the platform-specific initialization settings.
    ///
    /// *Windows Specific*:
    ///
    /// > When initializing for Windows platform, the HWND value returned in the
    /// > AkPlatformInitSettings structure is the foreground HWND at the moment of the
    /// > initialization of the sound engine and may not be the correct one for your need.
    /// >
    /// > Each game must specify the HWND that will be passed to DirectSound initialization.
    /// >
    /// > It is required that each game provides the correct HWND to be used or it could cause
    /// > one of the following problem:
    /// >> - Random Sound engine initialization failure.
    /// >> - Audio focus to be located on the wrong window.
    ///
    /// *Warning* This function is not thread-safe.
    ///
    /// *See also*
    /// > - [sound_engine::init](crate::sound_engine::init)
    /// > - [AkInitSettings::default]
    fn default() -> Self {
        let inner_settings = unsafe {
            let mut ss: crate::bindings::root::AkPlatformInitSettings = std::mem::zeroed();
            SoundEngine::GetDefaultPlatformInitSettings(&mut ss);
            ss
        };
        Self {
            #[cfg(windows)]
            h_wnd: (inner_settings.hWnd as *mut core::ffi::c_void).into(),
            thread_lengine: inner_settings.threadLEngine,
            thread_output_mgr: inner_settings.threadOutputMgr,
            thread_bank_manager: inner_settings.threadBankManager,
            thread_monitor: inner_settings.threadMonitor,
            num_refills_in_voice: inner_settings.uNumRefillsInVoice,
            sample_rate: inner_settings.uSampleRate,
            #[cfg(windows)]
            enable_avx_support: inner_settings.bEnableAvxSupport,
            #[cfg(windows)]
            max_system_audio_objects: inner_settings.uMaxSystemAudioObjects,
            #[cfg(target_os = "linux")]
            audio_api: inner_settings.eAudioAPI,
            #[cfg(target_os = "linux")]
            sample_type: inner_settings.sampleType,
        }
    }
}

impl AkInitSettings {
    /// When using DLLs for plugins, specify their path. Leave NULL if DLLs are in the same folder as the game executable.
    ///
    /// Note that on Windows, if `path` has spaces, the DLLs won't be discovered properly.
    pub fn with_plugin_dll_path<T: AsRef<str>>(mut self, path: T) -> Self {
        self.private_stuff.plugin_dll_path = to_os_char(path.as_ref());
        self
    }

    unsafe extern "C" fn ak_assert_hook(
        expression: *const std::os::raw::c_char,
        filename: *const std::os::raw::c_char,
        line_nb: std::os::raw::c_int,
    ) {
        use std::ffi::CStr;

        let expression = if expression.is_null() {
            "<no_expr>".to_string()
        } else {
            // Safety
            // in_pszExpression will be valid until to_string(), which will copy the bytes from
            // in_pszExpression onto the Rust-managed heap
            CStr::from_ptr(expression).to_str().unwrap().to_string()
        };
        let filename = if filename.is_null() {
            "<no_file>".to_string()
        } else {
            // Safety
            // in_pszFileName will be valid until to_string(), which will copy the bytes from
            // in_pszFileName onto the Rust-managed heap
            CStr::from_ptr(filename).to_str().unwrap().to_string()
        };

        error!("AK_ASSERT {}:{} on {}", filename, line_nb, expression);
    }

    pub(crate) fn as_ak(&mut self) -> crate::bindings::root::AkInitSettings {
        crate::bindings::root::AkInitSettings {
            pfnAssertHook: if self.install_assert_hook {
                Some(Self::ak_assert_hook)
            } else {
                None
            },
            uMaxNumPaths: self.max_num_paths,
            uCommandQueueSize: self.command_queue_size,
            bEnableGameSyncPreparation: self.enable_game_sync_preparation,
            uContinuousPlaybackLookAhead: self.continuous_playback_look_ahead,
            uNumSamplesPerFrame: self.num_samples_per_frame,
            uMonitorQueuePoolSize: self.monitor_queue_pool_size,
            settingsMainOutput: self.settings_main_output,
            uMaxHardwareTimeoutMs: self.max_hardware_timeout_ms,
            bUseSoundBankMgrThread: self.use_sound_bank_mgr_thread,
            bUseLEngineThread: self.use_lengine_thread,
            BGMCallback: self.bgm_callback,
            BGMCallbackCookie: self.bgm_callback_cookie.load(Ordering::SeqCst),
            szPluginDLLPath: self.private_stuff.plugin_dll_path.as_mut_ptr(),
            eFloorPlane: self.floor_plane,
            fGameUnitsToMeters: self.game_units_to_meters,
            uBankReadBufferSize: self.bank_read_buffer_size,
            fDebugOutOfRangeLimit: self.debug_out_of_range_limit,
            bDebugOutOfRangeCheckEnabled: self.debug_out_of_range_check_enabled,
            uCpuMonitorQueueMaxSize: self.monitor_queue_pool_size,
            settingsJobManager: self.settings_job_manager,
            fnProfilerPushTimer: None,
            fnProfilerPopTimer: None,
            fnProfilerPostMarker: None,
        }
    }
}

impl AkDeviceSettings {
    /// Gets the default values of the platform-independent initialization settings.
    pub(crate) fn as_ak(&mut self) -> crate::bindings::root::AkDeviceSettings {
        crate::bindings::root::AkDeviceSettings {
            pIOMemory: self.io_memory.load(Ordering::SeqCst),
            uIOMemorySize: self.io_memory_size,
            uIOMemoryAlignment: self.io_memory_alignment,
            ePoolAttributes: self.pool_attributes,
            uGranularity: self.granularity,
            threadProperties: self.thread_properties,
            fTargetAutoStmBufferLength: self.target_auto_stm_buffer_length,
            uMaxConcurrentIO: self.max_concurrent_io,
            bUseStreamCache: self.use_stream_cache,
            uMaxCachePinnedBytes: self.max_cache_pinned_bytes,
        }
    }
}

impl AkPlatformInitSettings {
    pub(crate) fn as_ak(&mut self) -> crate::bindings::root::AkPlatformInitSettings {
        crate::bindings::root::AkPlatformInitSettings {
            #[cfg(windows)]
            hWnd: self.h_wnd.load(Ordering::SeqCst) as *mut crate::bindings::root::HWND__,
            threadLEngine: self.thread_lengine,
            threadOutputMgr: self.thread_output_mgr,
            threadBankManager: self.thread_bank_manager,
            threadMonitor: self.thread_monitor,
            uNumRefillsInVoice: self.num_refills_in_voice,
            uSampleRate: self.sample_rate,
            #[cfg(windows)]
            bEnableAvxSupport: self.enable_avx_support,
            #[cfg(windows)]
            uMaxSystemAudioObjects: self.max_system_audio_objects,
            #[cfg(target_os = "linux")]
            eAudioAPI: self.audio_api,
            #[cfg(target_os = "linux")]
            sampleType: self.sample_type,
        }
    }
}
