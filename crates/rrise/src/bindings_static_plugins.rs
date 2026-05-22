/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::bindings::root::AK::PluginRegistration;
use paste::paste;

#[cfg(windows)]
macro_rules! import_plugin_registration {
    ($feature:ident, $global_var_name:ident) => {
        paste! {
            extern "C" {
                #[cfg(feature = "" $feature)]
                #[link_name = "\u{1}?" $global_var_name "Registration@@3VPluginRegistration@AK@@A"]
                pub(crate) static mut [<$global_var_name Registration>]: PluginRegistration;
            }
        }
    };
    ($feature:ident) => {
        paste! {
            extern "C" {
                #[cfg(feature = "" $feature)]
                #[link_name = "\u{1}?" $feature "Registration@@3VPluginRegistration@AK@@A"]
                pub(crate) static mut [<$feature Registration>]: PluginRegistration;
            }
        }
    };
}

#[cfg(not(windows))]
macro_rules! import_plugin_registration {
    ($feature:ident, $global_var_name:ident) => {
        paste! {
            #[cfg(feature = "" $feature)]
            extern "C" {
                pub(crate) static mut [<$global_var_name Registration>]: PluginRegistration;
            }
        }
    };
    ($feature:ident) => {
        paste! {
            #[cfg(feature = "" $feature)]
            extern "C" {
                pub(crate) static mut [<$feature Registration>]: PluginRegistration;
            }
        }
    };
}

import_plugin_registration![AkVorbisDecoder];
import_plugin_registration![AkOpusDecoder, AkOggOpusDecoder]; // see Ak/Plugin/AkOpusDecoderFactory.h
import_plugin_registration![AkOpusDecoder, AkWemOpusDecoder]; // see Ak/Plugin/AkOpusDecoderFactory.h
import_plugin_registration![AkMeterFX];
import_plugin_registration![AkAudioInputSource];
import_plugin_registration![AkCompressorFX];
import_plugin_registration![AkDelayFX];
import_plugin_registration![AkExpanderFX];
import_plugin_registration![AkFlangerFX];
import_plugin_registration![AkGainFX];
import_plugin_registration![AkGuitarDistortionFX];
import_plugin_registration![AkHarmonizerFX];
import_plugin_registration![AkMatrixReverbFX];
import_plugin_registration![AkParametricEQFX];
import_plugin_registration![AkPeakLimiterFX];
import_plugin_registration![AkPitchShifterFX];
import_plugin_registration![AkRecorderFX];
import_plugin_registration![AkRoomVerbFX];
import_plugin_registration![AkSilenceSource];
import_plugin_registration![AkSineSource, SineSource];
import_plugin_registration![AkStereoDelayFX];
import_plugin_registration![AkSynthOneSource, AkSynthOne];
import_plugin_registration![AkTimeStretchFX];
import_plugin_registration![AkToneSource];
import_plugin_registration![AkTremoloFX];
