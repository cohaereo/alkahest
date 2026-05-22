/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::bindings::root::AKRESULT;
use crate::bindings::root::AKRESULT::*;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[doc(hidden)]
impl Error for AKRESULT {}
#[doc(hidden)]
impl Display for AKRESULT {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", match self {
            AK_NotImplemented => "this feature is not implemented",
            AK_Success => "the operation was successful",
            AK_Fail => "the operation failed",
            AK_PartialSuccess => "the operation succeeded partially",
            AK_NotCompatible => "incompatible formats",
            AK_AlreadyConnected => "the stream is already connected to another node",
            AK_InvalidFile => "an unexpected value causes the file to be invalid",
            AK_AudioFileHeaderTooLarge => "the file header is too large",
            AK_MaxReached => "the maximum was reached",
            AK_InvalidID => "the ID is invalid",
            AK_IDNotFound => "the ID was not found",
            AK_InvalidInstanceID => "the InstanceID is invalid",
            AK_NoMoreData => "no more data is available from the source",
            AK_InvalidStateGroup => "the StateGroup is not a valid channel",
            AK_ChildAlreadyHasAParent => "the child already has a parent",
            AK_InvalidLanguage => "the language is invalid (applies to the Low-Level I/O)",
            AK_CannotAddItseflAsAChild => "it is not possible to add itself as its own child",
            AK_InvalidParameter => "something is not within bounds",
            AK_ElementAlreadyInList => "the item could not be added because it was already in the list",
            AK_PathNotFound => "this path is not known",
            AK_PathNoVertices => "stuff in vertices before trying to start it",
            AK_PathNotRunning => "only a running path can be paused",
            AK_PathNotPaused => "only a paused path can be resumed",
            AK_PathNodeAlreadyInList => "this path is already there",
            AK_PathNodeNotInList => "this path is not there",
            AK_DataNeeded => "the consumer needs more",
            AK_NoDataNeeded => "the consumer does not need more",
            AK_DataReady => "the provider has available data",
            AK_NoDataReady => "the provider does not have available data",
            AK_InsufficientMemory => "memory error",
            AK_Cancelled => "the requested action was cancelled (not an error)",
            AK_UnknownBankID => "trying to load a bank using an ID which is not defined",
            AK_BankReadError => "error while reading a bank",
            AK_InvalidSwitchType => "invalid switch type (used with the switch container)",
            AK_FormatNotReady => "source format not known yet",
            AK_WrongBankVersion => "the bank version is not compatible with the current bank reader",
            AK_FileNotFound => "file not found",
            AK_DeviceNotReady => "specified ID doesn't match a valid hardware device: either the device doesn't exist or is disabled",
            AK_BankAlreadyLoaded => "the bank load failed because the bank is already loaded",
            AK_RenderedFX => "the effect on the node is rendered",
            AK_ProcessNeeded => "a routine needs to be executed on some CPU",
            AK_ProcessDone => "the executed routine has finished its execution",
            AK_MemManagerNotInitialized => "the memory manager should have been initialized at this point",
            AK_StreamMgrNotInitialized => "the stream manager should have been initialized at this point",
            AK_SSEInstructionsNotSupported => "the machine does not support SSE instructions (required on PC)",
            AK_Busy => "the system is busy and could not process the request",
            AK_UnsupportedChannelConfig => "channel configuration is not supported in the current execution context",
            AK_PluginMediaNotAvailable => "plugin media is not available for effect",
            AK_MustBeVirtualized => "sound was Not Allowed to play",
            AK_CommandTooLarge => "SDK command is too large to fit in the command queue",
            AK_RejectedByFilter => "a play request was rejected due to the MIDI filter parameters",
            AK_InvalidCustomPlatformName => "detecting incompatibility between Custom platform of banks and custom platform of connected application",
            AK_DLLCannotLoad => "plugin DLL could not be loaded, either because it is not found or one dependency is missing",
            AK_DLLPathNotFound => "plugin DLL search path could not be found",
            AK_NoJavaVM => "no Java VM provided in AkInitSettings",
            AK_OpenSLError => "OpenSL returned an error.  Check error log for more details",
            AK_PluginNotRegistered => "plugin is not registered.  Make sure to implement a AK::PluginRegistration class for it and use AK_STATIC_LINK_PLUGIN in the game binary",
            AK_DataAlignmentError => "a pointer to audio data was not aligned to the platform's required alignment (check AkTypes.h in the platform-specific folder)",
            AK_DeviceNotCompatible => "incompatible Audio device",
            AK_DuplicateUniqueID => "two Wwise objects share the same ID",
            AK_InitBankNotLoaded => "the Init bank was not loaded yet, the sound engine isn't completely ready yet",
            AK_DeviceNotFound => "the specified device ID does not match with any of the output devices that the sound engine is currently using",
            AK_PlayingIDNotFound => "calling a function with a playing ID that is not known",
            AK_InvalidFloatValue => "one parameter has a invalid float value such as NaN, INF or FLT_MAX",
            AK_FileFormatMismatch => "media file format unexpected",
            AK_NoDistinctListener => "no distinct listener provided for AddOutput",
            AK_ACP_Error => "generic XMA decoder error",
            AK_ResourceInUse => "tesource is in use and cannot be released",
        })
    }
}
