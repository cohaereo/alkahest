#[macro_use]
extern crate log;

use std::sync::Arc;

use alkahest_data::sound::SRandomSound;
use alkahest_pm::{package_manager, PACKAGE_MANAGER};
use anyhow::Context;
use clap::Parser;
use destiny_pkg::{PackageManager, PackageVersion, TagHash};
use fs_err::File;
use tiger_parse::dpkg::PackageManagerExt;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
struct Args {
    /// Path to packages directory
    packages_path: String,

    /// List of SRandomSound hashes to extract, separated by commas
    randomsound_hashes: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::default()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    let args = Args::parse();

    info!("Initializing package manager");
    let pm = PackageManager::new(args.packages_path, PackageVersion::Destiny2Lightfall).unwrap();

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));

    std::fs::create_dir_all("sounds/extracted/")?;
    let hashes = args.randomsound_hashes.split(',').collect::<Vec<_>>();
    for hash in hashes {
        let hash = hash.trim();
        let hash = TagHash(u32::from_be(
            u32::from_str_radix(hash, 16).context("Invalid hash format")?,
        ));
        let randomsound: SRandomSound = package_manager().read_tag_struct(hash)?;
        info!(
            "Extracting RandomSound {}, {} streams",
            hash,
            randomsound.streams.len()
        );

        let sbox_sound = SboxSound {
            sounds: randomsound
                .streams
                .iter()
                .map(|s| format!("sounds/extracted/{s}.vsnd"))
                .collect(),
            ..Default::default()
        };

        std::fs::write(
            format!("sounds/{}.sound", hash),
            serde_json::to_string_pretty(&sbox_sound)?,
        )?;

        for stream in randomsound.streams {
            let sound_data = package_manager().read_tag(stream)?;
            let filename = format!(".\\{stream}.wem");

            let (samples, desc) = match vgmstream::read_file_to_samples(&sound_data, Some(filename))
            {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to decode audio file: {e}");
                    continue;
                }
            };

            if let Ok(mut f) = File::create(&format!("sounds/extracted/{stream}.wav")) {
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
            }
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SboxSound {
    #[serde(rename = "UI")]
    pub ui: bool,
    pub volume: String,
    pub pitch: String,
    pub decibels: i64,
    pub selection_mode: String,
    pub sounds: Vec<String>,
    #[serde(rename = "__version")]
    pub version: i64,
    #[serde(rename = "__references")]
    pub references: Vec<()>,
}

impl Default for SboxSound {
    fn default() -> Self {
        Self {
            ui: false,
            volume: "1.00,1.00,0".to_string(),
            pitch: "1.00,1.00,0".to_string(),
            decibels: 70,
            selection_mode: "Random".to_string(),
            sounds: vec![],
            version: 0,
            references: vec![],
        }
    }
}
