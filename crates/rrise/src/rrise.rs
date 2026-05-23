/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

#![doc = include_str!("../README.md")]

#[cfg(not(wwrelease))]
pub mod communication;
pub mod game_syncs;
pub mod memory_mgr;
pub mod music_engine;
pub mod query_params;
pub mod settings;
pub mod sound_engine;
pub mod spatial_audio;
pub mod stream_mgr;

mod bindings;
mod bindings_static_plugins;
mod error;
mod transform;

use std::fmt::{Debug, Display, Formatter};

/// Acoustic Texture ID
pub use bindings::root::AkAcousticTextureID;
/// Argument value ID
pub use bindings::root::AkArgumentValueID;
#[cfg(target_os = "linux")]
#[doc(inline)]
pub use bindings::root::AkAudioAPI;
/// Audio Object ID
pub use bindings::root::AkAudioObjectID;
/// Auxilliary bus ID
pub use bindings::root::AkAuxBusID;
/// Run time bank ID
pub use bindings::root::AkBankID;
#[doc(inline)]
pub use bindings::root::AkCallbackType;
#[doc(inline)]
pub use bindings::root::AkChannelConfig;
/// Channel mask (similar to WAVE_FORMAT_EXTENSIBLE). Bit values are defined in AkSpeakerConfig.h.
pub use bindings::root::AkChannelMask;
/// Codec plug-in ID
pub use bindings::root::AkCodecID;
#[doc(inline)]
pub use bindings::root::AkCurveInterpolation;
/// Data compression format ID
pub use bindings::root::AkDataCompID;
/// Data interleaved state ID
pub use bindings::root::AkDataInterleaveID;
/// Data sample type ID
pub use bindings::root::AkDataTypeID;
/// I/O device ID
pub use bindings::root::AkDeviceID;
/// Integer-type file identifier
pub use bindings::root::AkFileID;
/// Game object ID
pub use bindings::root::AkGameObjectID;
/// Image Source ID
pub use bindings::root::AkImageSourceID;
/// Low-pass filter type
pub use bindings::root::AkLPFType;
#[doc(inline)]
pub use bindings::root::AkListenerPosition;
/// Memory pool ID
pub use bindings::root::AkMemPoolId;
/// MIDI channel number, usually 0-15.
pub use bindings::root::AkMidiChannelNo;
/// Modulator ID
pub use bindings::root::AkModulatorID;
/// Audio Output device ID
pub use bindings::root::AkOutputDeviceID;
#[doc(inline)]
pub use bindings::root::AkOutputSettings;
#[doc(inline)]
pub use bindings::root::AkPanningRule;
/// Unique node (bus, voice) identifier for profiling.
pub use bindings::root::AkPipelineID;
/// Pitch value
pub use bindings::root::AkPitchValue;
/// Playing ID
pub use bindings::root::AkPlayingID;
/// Source or effect plug-in ID
pub use bindings::root::AkPluginID;
/// Source or effect plug-in parameter ID
pub use bindings::root::AkPluginParamID;
/// Port number
pub use bindings::root::AkPortNumber;
/// Priority
pub use bindings::root::AkPriority;
/// Unique (per emitter) identifier for an emitter-listener ray.
pub use bindings::root::AkRayID;
#[doc(inline)]
pub use bindings::root::AkReal32;
/// Real time parameter control ID
pub use bindings::root::AkRtpcID;
/// Real time parameter control value
pub use bindings::root::AkRtpcValue;
#[doc(inline)]
pub use bindings::root::AkSegmentInfo;
#[doc(inline)]
pub use bindings::root::AkSoundPosition;
/// State group ID
pub use bindings::root::AkStateGroupID;
/// State ID
pub use bindings::root::AkStateID;
/// Switch group ID
pub use bindings::root::AkSwitchGroupID;
/// Switch ID
pub use bindings::root::AkSwitchStateID;
#[doc(inline)]
pub use bindings::root::AkThreadProperties;
/// Time in ms
pub use bindings::root::AkTimeMs;
#[doc(inline)]
pub use bindings::root::AkTransform;
/// Trigger ID
pub use bindings::root::AkTriggerID;
#[doc(inline)]
pub use bindings::root::AkUInt32;
/// Unique 32-bit ID
pub use bindings::root::AkUniqueID;
/// Volume value( also apply to LFE )
pub use bindings::root::AkVolumeValue;
#[doc(inline)]
pub use bindings::root::AKRESULT as AkResult;
#[doc(inline)]
pub use bindings::root::AK_DEFAULT_BANK_IO_PRIORITY;
#[doc(inline)]
pub use bindings::root::AK_DEFAULT_BANK_THROUGHPUT;
#[doc(inline)]
pub use bindings::root::AK_DEFAULT_POOL_ID;
#[doc(inline)]
pub use bindings::root::AK_DEFAULT_PRIORITY;
#[doc(inline)]
pub use bindings::root::AK_DEFAULT_SWITCH_STATE;
#[doc(inline)]
pub use bindings::root::AK_FALLBACK_ARGUMENTVALUE_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_AUX_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_BANK_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_CHANNELMASK;
#[doc(inline)]
pub use bindings::root::AK_INVALID_DEVICE_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_FILE_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_OUTPUT_DEVICE_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_PIPELINE_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_PLAYING_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_PLUGINID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_POOL_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_RTPC_ID;
#[doc(inline)]
pub use bindings::root::AK_INVALID_UNIQUE_ID;
#[doc(inline)]
pub use bindings::root::AK_MAX_PRIORITY;
#[doc(inline)]
pub use bindings::root::AK_MIN_PRIORITY;
#[doc(inline)]
pub use bindings::root::AK_SOUNDBANK_VERSION;
#[doc(inline)]
pub use bindings::root::{AkVector, AkVector64};
#[doc(inline)]
pub use bindings::AK_INVALID_AUDIO_OBJECT_ID;
#[doc(inline)]
pub use bindings::AK_INVALID_GAME_OBJECT;
pub use error::*;
pub use transform::*;

pub use crate::bindings::root::{
    AkMIDIEvent_tCc, AkMIDIEvent_tChanAftertouch, AkMIDIEvent_tGen, AkMIDIEvent_tNoteAftertouch,
    AkMIDIEvent_tNoteOnOff, AkMIDIEvent_tPitchBend, AkMIDIEvent_tProgramChange,
    AkMIDIEvent_tWwiseCmd,
};

#[derive(Debug, Copy, Clone)]
/// An ID for functions that can take either a string or numerical identifier for Wwise objects.
pub enum AkID<'a> {
    Name(&'a str),
    ID(AkUniqueID),
}

impl<'a> Display for AkID<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(s) => write!(f, "{}", s),
            Self::ID(id) => write!(f, "{}", id),
        }
    }
}

impl<'a> From<AkUniqueID> for AkID<'a> {
    fn from(id: AkUniqueID) -> Self {
        Self::ID(id)
    }
}

impl<'a> From<&'a str> for AkID<'a> {
    fn from(name: &'a str) -> Self {
        Self::Name(name)
    }
}

impl<'a> From<&'a String> for AkID<'a> {
    fn from(name: &'a String) -> Self {
        Self::Name(name.as_str())
    }
}

#[doc(hidden)]
pub(crate) type OsChar = bindings::root::AkOSChar;

#[doc(hidden)]
#[macro_export]
macro_rules! with_cstring {
    ($($text:expr => $tmp:ident),+ { $($stmt:stmt)+ }) => {
        {
            use ::std::ffi::CString;
            $(
            let $tmp = CString::new($text).expect("text shouldn't contain null bytes");
            )+
            $($stmt)+
        }
    };
}

#[doc(hidden)]
/// Create a copy of str as a vector of OsChar (u16 on Windows, i8 == c_char on other platforms).
pub(crate) fn to_os_char<T: AsRef<str>>(str: T) -> Vec<OsChar> {
    #[cfg(windows)]
    {
        // On windows, AkOsChar* ~~ Vec<u16>
        use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
        OsStr::new(str.as_ref())
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect()
    }

    #[cfg(not(windows))]
    {
        use ::std::ffi::CString;
        CString::new(str.as_ref())
            .expect("str shouldn't contain null bytes")
            .as_bytes_with_nul()
            .iter()
            .map(|c| *c as OsChar)
            .collect()
    }
}

#[doc(hidden)]
/// Wraps an unsafe call to Wwise and match its result to a Result<(), AkResult>.
///
/// For example, `ak_call_result[RenderAudio(allow_sync_render)]` expands to
/// ```rust,ignore
/// match unsafe { RenderAudio(allow_sync_render) } {
///     AkResult::AK_Success => Ok(()),
///     error_code => Err(error_code)
/// }
/// ```
#[macro_export]
macro_rules! ak_call_result {
    ($the_call:expr) => {
        match unsafe { $the_call } {
            AkResult::AK_Success => Ok(()),
            error_code => Err(error_code)
        }
    };
    ($the_call:expr => ($($the_tupled_result:expr),*)) => {
        match unsafe { $the_call } {
            AkResult::AK_Success => Ok(($($the_tupled_result),*)),
            error_code => Err(error_code)
        }
    };
    ($the_call:expr => $the_result:expr) => {
        match unsafe { $the_call } {
            AkResult::AK_Success => Ok($the_result),
            error_code => Err(error_code)
        }
    };
}

#[derive(Debug, Copy, Clone)]
/// Description of a MIDI event
pub enum AkMIDIEvent {
    /// Controller event
    Cc(AkMidiChannelNo, AkMIDIEvent_tCc),
    /// Channel after touch event
    ChanAftertouch(AkMidiChannelNo, AkMIDIEvent_tChanAftertouch),
    /// Generic event (ie, if no other variant apply)
    Gen(AkMidiChannelNo, AkMIDIEvent_tGen),
    /// Note after touch event
    NoteAftertouch(AkMidiChannelNo, AkMIDIEvent_tNoteAftertouch),
    /// Note became On event
    NoteOn(AkMidiChannelNo, AkMIDIEvent_tNoteOnOff),
    /// Note became Off event
    NoteOff(AkMidiChannelNo, AkMIDIEvent_tNoteOnOff),
    /// Pitch bent event
    PitchBend(AkMidiChannelNo, AkMIDIEvent_tPitchBend),
    /// Program change event
    ProgramChange(AkMidiChannelNo, AkMIDIEvent_tProgramChange),
    /// Wwise command event
    WwiseCmd(AkMidiChannelNo, AkMIDIEvent_tWwiseCmd),
}

impl AkMIDIEvent {
    pub fn is_wwise_cmd(&self) -> bool {
        matches!(self, Self::WwiseCmd(_, _))
    }

    pub fn is_wwise_cmd_play(&self) -> bool {
        match self {
            Self::WwiseCmd(_, wwise_cmd) => {
                wwise_cmd.uCmd == bindings::root::AK_MIDI_WWISE_CMD_PLAY as u16
            }
            _ => false,
        }
    }

    pub fn is_wwise_cmd_pause(&self) -> bool {
        match self {
            Self::WwiseCmd(_, wwise_cmd) => {
                wwise_cmd.uCmd == bindings::root::AK_MIDI_WWISE_CMD_PAUSE as u16
            }
            _ => false,
        }
    }

    pub fn is_wwise_cmd_stop(&self) -> bool {
        match self {
            Self::WwiseCmd(_, wwise_cmd) => {
                wwise_cmd.uCmd == bindings::root::AK_MIDI_WWISE_CMD_STOP as u16
            }
            _ => false,
        }
    }

    pub fn is_wwise_cmd_resume(&self) -> bool {
        match self {
            Self::WwiseCmd(_, wwise_cmd) => {
                wwise_cmd.uCmd == bindings::root::AK_MIDI_WWISE_CMD_RESUME as u16
            }
            _ => false,
        }
    }

    pub fn is_wwise_cmd_seek_ms(&self) -> bool {
        match self {
            Self::WwiseCmd(_, wwise_cmd) => {
                wwise_cmd.uCmd == bindings::root::AK_MIDI_WWISE_CMD_SEEK_MS as u16
            }
            _ => false,
        }
    }

    pub fn is_wwise_cmd_seek_samples(&self) -> bool {
        match self {
            Self::WwiseCmd(_, wwise_cmd) => {
                wwise_cmd.uCmd == bindings::root::AK_MIDI_WWISE_CMD_SEEK_SAMPLES as u16
            }
            _ => false,
        }
    }

    pub fn is_wwise_cmd_seek(&self) -> bool {
        self.is_wwise_cmd_seek_ms() || self.is_wwise_cmd_seek_samples()
    }

    pub fn is_wwise_cmd_known(&self) -> bool {
        self.is_wwise_cmd_play()
            || self.is_wwise_cmd_pause()
            || self.is_wwise_cmd_resume()
            || self.is_wwise_cmd_stop()
            || self.is_wwise_cmd_seek()
    }

    pub fn is_note_on(&self) -> bool {
        match self {
            Self::NoteOn(_, note_on_off) => note_on_off.byVelocity == 0,
            _ => false,
        }
    }

    pub fn is_note_off(&self) -> bool {
        match self {
            Self::NoteOff(_, note_on_off) => note_on_off.byVelocity == 0,
            _ => false,
        }
    }
}

impl From<bindings::root::AkMIDIEvent> for AkMIDIEvent {
    fn from(e: bindings::root::AkMIDIEvent) -> Self {
        unsafe {
            match e.byType as u32 {
                bindings::root::AK_MIDI_EVENT_TYPE_NOTE_OFF => {
                    AkMIDIEvent::NoteOn(e.byChan, e.__bindgen_anon_1.NoteOnOff)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_NOTE_ON => {
                    AkMIDIEvent::NoteOff(e.byChan, e.__bindgen_anon_1.NoteOnOff)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_NOTE_AFTERTOUCH => {
                    AkMIDIEvent::NoteAftertouch(e.byChan, e.__bindgen_anon_1.NoteAftertouch)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_CONTROLLER => {
                    AkMIDIEvent::Cc(e.byChan, e.__bindgen_anon_1.Cc)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_PROGRAM_CHANGE => {
                    AkMIDIEvent::ProgramChange(e.byChan, e.__bindgen_anon_1.ProgramChange)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_CHANNEL_AFTERTOUCH => {
                    AkMIDIEvent::ChanAftertouch(e.byChan, e.__bindgen_anon_1.ChanAftertouch)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_PITCH_BEND => {
                    AkMIDIEvent::PitchBend(e.byChan, e.__bindgen_anon_1.PitchBend)
                }
                bindings::root::AK_MIDI_EVENT_TYPE_WWISE_CMD => {
                    AkMIDIEvent::WwiseCmd(e.byChan, e.__bindgen_anon_1.WwiseCmd)
                }
                _ => AkMIDIEvent::Gen(e.byChan, e.__bindgen_anon_1.Gen),
            }
        }
    }
}

#[derive(Debug, Clone)]
/// Callback information used for all notifications sent from Wwise.
pub enum AkCallbackInfo {
    /// Basic information structure returned for notifications that are not handled by another variant.
    Default {
        game_obj_id: AkGameObjectID,
        callback_type: AkCallbackType,
    },

    /// Callback information structure corresponding to [AkCallbackType::AK_MusicSyncEntry],
    /// [AkCallbackType::AK_MusicSyncBeat], [AkCallbackType::AK_MusicSyncBar],
    /// [AkCallbackType::AK_MusicSyncExit], [AkCallbackType::AK_MusicSyncGrid],
    /// [AkCallbackType::AK_MusicSyncPoint] and [AkCallbackType::AK_MusicSyncUserCue].
    ///
    /// If you need the Tempo, you can compute it using the `fBeatDuration`:
    /// Tempo (beats per minute) = 60/`fBeatDuration`
    MusicSync {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Segment information corresponding to the segment triggering this callback
        segment_info: AkSegmentInfo,
        /// Would be either [AkCallbackType::AK_MusicSyncEntry],
        /// [AkCallbackType::AK_MusicSyncBeat], [AkCallbackType::AK_MusicSyncBar],
        /// [AkCallbackType::AK_MusicSyncExit], [AkCallbackType::AK_MusicSyncGrid],
        /// [AkCallbackType::AK_MusicSyncPoint] or [AkCallbackType::AK_MusicSyncUserCue]
        music_sync_type: AkCallbackType,
        /// Cue name. Set for notifications [AkCallbackType::AK_MusicSyncUserCue]. Empty if cue has no name.
        user_cue_name: String,
    },

    #[non_exhaustive]
    /// Callback information structure corresponding to [AkCallbackType::AK_EndOfDynamicSequenceItem].
    DynamicSequenceItem {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Dynamic Sequence, returned by [DynamicSequence::Open()](sound_engine::DynamicSequence::Open)
        playing_id: AkPlayingID,
        /// Audio Node ID of finished item
        audio_node_id: AkUniqueID,
        // TODO: custom_info cookie
    },

    /// Callback information structure corresponding to [AkCallbackType::AK_EndOfEvent],
    /// [AkCallbackType::AK_MusicPlayStarted] and [AkCallbackType::AK_Starvation].
    Event {
        game_obj_id: AkGameObjectID,
        /// Would be either [AkCallbackType::AK_EndOfEvent], [AkCallbackType::AK_MusicPlayStarted]
        /// or [AkCallbackType::AK_Starvation]
        callback_type: AkCallbackType,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Unique ID of Event, passed to [PostEvent::new()](sound_engine::PostEvent::new)
        event_id: AkUniqueID,
    },

    /// Callback information structure corresponding to [AkCallbackType::AK_Duration].
    Duration {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Unique ID of Event, passed to [PostEvent::new()](sound_engine::PostEvent::new)
        event_id: AkUniqueID,
        /// Duration of the sound (unit: milliseconds)
        duration: AkReal32,
        /// Estimated duration of the sound depending on source settings such as pitch. (unit: milliseconds)
        estimated_duration: AkReal32,
        /// Audio Node ID of playing item
        audio_node_id: AkUniqueID,
        /// Media ID of playing item. (corresponds to 'ID' attribute of 'File' element in SoundBank metadata file)
        media_id: AkUniqueID,
        /// True if source is streaming, false otherwise
        streaming: bool,
    },

    /// Callback information structure corresponding to [AkCallbackType::AK_Marker].
    Marker {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Unique ID of Event, passed to [PostEvent::new()](sound_engine::PostEvent::new)
        event_id: AkUniqueID,
        /// Cue point identifier
        identifier: AkUniqueID,
        /// Position in the cue point (unit: sample frames)
        position: AkUInt32,
        /// Label of the marker, read from the file
        label: String,
    },

    /// Callback information structure corresponding to [AkCallbackType::AK_MIDIEvent].
    Midi {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Unique ID of Event, passed to [PostEvent::new()](sound_engine::PostEvent::new)
        event_id: AkUniqueID,
        /// MIDI event triggered by event
        midi_event: AkMIDIEvent,
    },

    /// Callback information structure corresponding to [AkCallbackType::AK_MusicPlaylistSelect].
    ///
    /// Called when a music playlist container must select its next item to play.
    /// The members `playlist_selection` and `playlist_item_done` are set by the sound
    /// engine before the callback function call. They are set to the next item
    /// selected by the sound engine.
    MusicPlaylist {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Unique ID of Event, passed to [PostEvent::new()](sound_engine::PostEvent::new)
        event_id: AkUniqueID,
        /// ID of playlist node
        playlist_id: AkUniqueID,
        /// Number of items in playlist node (may be segments or other playlists)
        num_playlist_items: AkUInt32,
        /// Selection: set by sound engine
        playlist_selection: AkUInt32,
        /// Playlist node done: set by sound engine
        playlist_item_done: AkUInt32,
    },

    #[non_exhaustive]
    /// Callback information structure corresponding to [AkCallbackType::AK_SpeakerVolumeMatrix],
    /// and passed to callbacks registered in [RegisterBusVolumeCallback()] or
    /// [PostEvent](sound_engine::PostEvent) with [AkCallbackType::AK_SpeakerVolumeMatrix].
    ///
    /// These callbacks are called at every audio frame for every connection from an input (voice
    /// or bus) to an output bus (standard or auxiliary), at the point when an input signal is about to be mixed into a mixing bus, but just before
    /// having been scaled in accordance to volumes authored in Wwise. The volumes are passed via this structure as pointers because they can be modified
    /// in the callbacks. They are factored into two linear values ([0..1]): a common base value (pfBaseVolume), that is channel-agnostic and represents
    /// the collapsed gain of all volume changes in Wwise (sliders, actions, RTPC, attenuations, ...), and a matrix of gains per input/output channel,
    /// which depends on spatialization. Use the methods of AK::SpeakerVolumes::Matrix, defined in AkCommonDefs.h, to perform operations on them.
    /// Access each input channel of the volumes matrix with AK::SpeakerVolumes::Matrix::GetChannel(), passing it the input and output channel configuration.
    /// Then, you may access each element of the output vector using the standard bracket [] operator. See AK::SpeakerVolumes for more details.
    /// It is crucial that the processing done in the callback be lightweight and non-blocking.
    SpeakerMatrixVolume {
        game_obj_id: AkGameObjectID,
        /// Playing ID of Event, returned by [PostEvent::post()](sound_engine::PostEvent::post)
        playing_id: AkPlayingID,
        /// Unique ID of Event, passed to [PostEvent::new()](sound_engine::PostEvent::new)
        event_id: AkUniqueID,
        /// Channel configuration of the voice/bus
        input_config: AkChannelConfig,
        /// Channel configuration of the output bus
        output_config: AkChannelConfig,
        // TODO: volumes
        // TODO: base_volume
        // TODO: emitter_listener_volume
        // TODO: context
        // TODO: mixer_context
    },
    // TODO: BusMetering
    // TODO: OutputDeviceMetering
}

impl AkCallbackType {
    /// Checks whether this bitflag has at least one of the bits in `flags` set.
    pub fn contains(self, flags: Self) -> bool {
        (self & flags).0 > Self(0).0
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AkCallbackType {
    fn default() -> Self {
        Self(0)
    }
}

impl Display for AkCallbackType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut flags = vec![];

        if self.contains(AkCallbackType::AK_EndOfEvent) {
            flags.push("AK_EndOfEvent")
        }
        if self.contains(AkCallbackType::AK_EndOfDynamicSequenceItem) {
            flags.push("AK_EndOfDynamicSequenceItem")
        }
        if self.contains(AkCallbackType::AK_Marker) {
            flags.push("AK_Marker")
        }
        if self.contains(AkCallbackType::AK_Duration) {
            flags.push("AK_Duration")
        }
        if self.contains(AkCallbackType::AK_SpeakerVolumeMatrix) {
            flags.push("AK_SpeakerVolumeMatrix")
        }
        if self.contains(AkCallbackType::AK_Starvation) {
            flags.push("AK_Starvation")
        }
        if self.contains(AkCallbackType::AK_MusicPlaylistSelect) {
            flags.push("AK_MusicPlaylistSelect")
        }
        if self.contains(AkCallbackType::AK_MusicPlayStarted) {
            flags.push("AK_MusicPlayStarted")
        }
        if self.contains(AkCallbackType::AK_MusicSyncBeat) {
            flags.push("AK_MusicSyncBeat")
        }
        if self.contains(AkCallbackType::AK_MusicSyncBar) {
            flags.push("AK_MusicSyncBar")
        }
        if self.contains(AkCallbackType::AK_MusicSyncEntry) {
            flags.push("AK_MusicSyncEntry")
        }
        if self.contains(AkCallbackType::AK_MusicSyncExit) {
            flags.push("AK_MusicSyncExit")
        }
        if self.contains(AkCallbackType::AK_MusicSyncGrid) {
            flags.push("AK_MusicSyncGrid")
        }
        if self.contains(AkCallbackType::AK_MusicSyncUserCue) {
            flags.push("AK_MusicSyncUserCue")
        }
        if self.contains(AkCallbackType::AK_MusicSyncPoint) {
            flags.push("AK_MusicSyncPoint")
        }
        if self.contains(AkCallbackType::AK_MIDIEvent) {
            flags.push("AK_MIDIEvent")
        }
        if self.contains(AkCallbackType::AK_EnableGetSourcePlayPosition) {
            flags.push("AK_EnableGetSourcePlayPosition")
        }
        if self.contains(AkCallbackType::AK_EnableGetMusicPlayPosition) {
            flags.push("AK_EnableGetMusicPlayPosition")
        }
        if self.contains(AkCallbackType::AK_EnableGetSourceStreamBuffering) {
            flags.push("AK_EnableGetSourceStreamBuffering")
        }

        write!(
            f,
            "{}",
            if flags.is_empty() {
                format!("<UnknownAkCallbackType:{}>", self.0)
            } else {
                flags.join(" | ")
            }
        )
    }
}
