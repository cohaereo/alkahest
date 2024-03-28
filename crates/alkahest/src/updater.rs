use std::io::Cursor;

use anyhow::Context;
use poll_promise::Promise;
use serde::{Deserialize, Serialize};

use crate::{
    icons::{ICON_CANCEL, ICON_LIGHTNING_BOLT, ICON_SHIELD_HALF_FULL},
    util::{changelog_diff::parse_changelog, consts, version_fixup},
};

#[derive(Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub download_url: String,
    pub url: String,
    pub changelog: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Nightly,
    Disabled,
}

impl UpdateChannel {
    pub async fn check_for_updates(&self) -> anyhow::Result<Option<AvailableUpdate>> {
        info!("Checking for updates on channel {self:?}");
        match self {
            UpdateChannel::Stable => check_stable_release().await,
            UpdateChannel::Nightly => check_nightly_release().await,
            UpdateChannel::Disabled => Ok(None),
        }
    }

    pub fn icon(&self) -> char {
        match self {
            UpdateChannel::Stable => ICON_SHIELD_HALF_FULL,
            UpdateChannel::Nightly => ICON_LIGHTNING_BOLT,
            UpdateChannel::Disabled => ICON_CANCEL,
        }
    }
}

const REPOSITORY: &str = "cohaereo/alkahest";

const API_ENDPOINT: &str = "https://api.github.com";
const GET_RELEASES: &str = "/repos/%/releases";
const GET_NIGHTLY_RUNS: &str =
    "/repos/%/actions/workflows/build-nightly.yml/runs?status=success&per_page=3";
const GET_ARTIFACT_FOR_RUN: &str =
    "https://nightly.link/cohaereo/alkahest/actions/runs/%/alkahest.zip";

async fn github_get<P: AsRef<str>>(path: P) -> anyhow::Result<reqwest::Response> {
    let url = format!("{}{}", API_ENDPOINT, path.as_ref());
    let response = reqwest::Client::builder()
        .build()?
        .get(url)
        .header("User-Agent", "alkahest")
        .header("Accept", "application/vnd.github.v3+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    Ok(response)
}

pub async fn check_nightly_release() -> anyhow::Result<Option<AvailableUpdate>> {
    #[derive(Deserialize)]
    struct WorkflowRuns {
        // pub total_count: usize,
        pub workflow_runs: Vec<WorkflowRunPartial>,
    }

    #[derive(Deserialize)]
    struct WorkflowRunPartial {
        pub id: u64,
        pub head_sha: String,
        pub html_url: String,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    let response = github_get(GET_NIGHTLY_RUNS.replace('%', REPOSITORY))
        .await?
        .error_for_status()?;
    let runs: WorkflowRuns = response.json().await?;

    let run = runs
        .workflow_runs
        .into_iter()
        .next()
        .context("No nightly runs found")?;
    let current_build_date = chrono::DateTime::parse_from_rfc3339(consts::BUILD_TIMESTAMP)
        .expect("Failed to parse BUILD_TIMESTAMP");

    if run.created_at < (current_build_date + chrono::Duration::minutes(5)) {
        return Ok(None);
    }

    let changelog_url = format!(
        "https://raw.githubusercontent.com/{}/{}/CHANGELOG.md",
        REPOSITORY, run.head_sha
    );
    let changelog = match reqwest::get(changelog_url).await {
        Ok(changelog) => changelog.text().await.ok(),
        Err(e) => {
            error!("Failed to retrieve changelog: {}", e);
            None
        }
    };

    let changelog = match changelog {
        Some(changelog) => {
            let current_changelog = parse_changelog(consts::CHANGELOG_MD);
            let new_changelog = parse_changelog(&changelog);

            if let Some(new_version) = new_changelog.first() {
                if let Some(current_version) = current_changelog.first() {
                    let diff = new_version.diff(current_version);
                    diff.to_string()
                } else {
                    new_version.to_string()
                }
            } else {
                "No changes found in changelog".to_string()
            }
        }
        None => "Failed to retrieve changelog".to_string(),
    };

    Ok(Some(AvailableUpdate {
        version: format!("Nightly ({})", run.created_at.format("%d/%m/%Y %H:%M")),
        download_url: GET_ARTIFACT_FOR_RUN.replace('%', &run.id.to_string()),
        url: run.html_url,
        changelog,
    }))
}

pub async fn check_stable_release() -> anyhow::Result<Option<AvailableUpdate>> {
    #[derive(Deserialize)]
    struct ReleasePartial {
        pub tag_name: String,
        pub name: String,
        pub html_url: String,

        pub assets: Vec<AssetPartial>,
        pub body: String,
    }

    #[derive(Deserialize)]
    struct AssetPartial {
        pub name: String,
        pub browser_download_url: String,
    }

    let response = github_get(GET_RELEASES.replace('%', REPOSITORY))
        .await?
        .error_for_status()?;
    let releases: Vec<ReleasePartial> = response.json().await?;

    let release = releases.into_iter().next().context("No releases found")?;
    let release_semver = semver::Version::parse(&version_fixup(&release.tag_name))?;
    let current_semver = semver::Version::parse(&version_fixup(consts::VERSION))?;

    if release_semver <= current_semver {
        return Ok(None);
    }

    let download_url = release
        .assets
        .iter()
        .find(|asset| asset.name == "alkahest.zip")
        .map(|asset| asset.browser_download_url.clone())
        .context("alkahest.zip not found in release")?;

    Ok(Some(AvailableUpdate {
        version: release.name,
        download_url,
        url: release.html_url,
        changelog: release.body,
    }))
}

#[derive(Default)]
pub struct UpdateCheck(pub Option<Promise<Option<AvailableUpdate>>>);

impl UpdateCheck {
    pub fn start(&mut self, channel: UpdateChannel) {
        self.0 = Some(Promise::spawn_async(async move {
            match channel.check_for_updates().await {
                Ok(o) => o,
                Err(e) => {
                    error!("Failed to check for updates: {}", e);
                    None
                }
            }
        }));
    }
}

pub fn execute_update(zip_data: Vec<u8>) -> anyhow::Result<()> {
    let exe_path = std::env::current_exe().context("Failed to retrieve current executable path")?;
    let old_exe_path = exe_path.with_file_name("alkahest_old.exe");

    std::fs::rename(&exe_path, old_exe_path)
        .context("Failed to move the old alkahest executable")?;

    let mut zip_reader = Cursor::new(zip_data);
    let exe_dir = exe_path
        .parent()
        .context("Exe does not have a parent directory??")?;
    zip_extract::extract(&mut zip_reader, exe_dir, true)?;

    if !exe_path.exists() {
        return Err(anyhow::anyhow!("alkahest.exe does not exist in the zip"));
    }

    // Spawn the new process
    std::process::Command::new(exe_path)
        .args(std::env::args().skip(1))
        .spawn()
        .context("Failed to spawn the new alkahest process")?;

    std::process::exit(0);
}
