use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::util::{changelog_diff::parse_changelog, consts, version_fixup};

#[derive(Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub download_url: String,
    pub url: String,
    pub changelog: String,
}

#[derive(Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Nightly,
    Disabled,
}

impl UpdateChannel {
    pub fn check_for_updates(&self) -> anyhow::Result<Option<AvailableUpdate>> {
        match self {
            UpdateChannel::Stable => check_stable_release(),
            UpdateChannel::Nightly => check_nightly_release(),
            UpdateChannel::Disabled => Ok(None),
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

fn github_get<P: AsRef<str>>(path: P) -> anyhow::Result<reqwest::blocking::Response> {
    let url = format!("{}{}", API_ENDPOINT, path.as_ref());
    let response = reqwest::blocking::Client::builder()
        .build()?
        .get(url)
        .header("User-Agent", "alkahest")
        .header("Accept", "application/vnd.github.v3+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()?;

    Ok(response)
}

pub fn check_nightly_release() -> anyhow::Result<Option<AvailableUpdate>> {
    #[derive(Deserialize)]
    struct WorkflowRuns {
        // pub total_count: usize,
        pub workflow_runs: Vec<WorkflowRunPartial>,
    }

    #[derive(Deserialize)]
    struct WorkflowRunPartial {
        pub id: u64,
        pub head_sha: String,
        pub url: String,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    let response = github_get(GET_NIGHTLY_RUNS.replace('%', REPOSITORY))?.error_for_status()?;
    let runs: WorkflowRuns = response.json()?;

    let run = runs
        .workflow_runs
        .into_iter()
        .next()
        .context("No nightly runs found")?;
    let current_build_date = chrono::DateTime::parse_from_rfc3339(consts::BUILD_TIMESTAMP)
        .expect("Failed to parse BUILD_TIMESTAMP");

    // if run.created_at < (current_build_date + chrono::Duration::minutes(5)) {
    //     return None;
    // }

    let changelog_url = format!(
        "https://raw.githubusercontent.com/{}/{}/CHANGELOG.md",
        REPOSITORY, run.head_sha
    );
    let changelog = match reqwest::blocking::get(changelog_url) {
        Ok(changelog) => changelog.text().ok(),
        Err(e) => {
            error!("Failed to retrieve changelog: {}", e);
            None
        }
    };

    let changelog = match changelog {
        Some(changelog) => {
            let current_changelog = parse_changelog(consts::CHANGELOG_MD);
            println!("current: {:#?}", current_changelog);
            let new_changelog = parse_changelog(&changelog);
            println!("new: {:#?}", new_changelog);

            if let Some(new_version) = new_changelog.first() {
                if let Some(current_version) = current_changelog.first() {
                    let diff = current_version.diff(new_version);
                    diff.to_string()
                } else {
                    new_version.to_string()
                }
            } else {
                "No changes?".to_string()
            }
        }
        None => "Failed to retrieve changelog".to_string(),
    };

    Ok(Some(AvailableUpdate {
        version: format!("Nightly ({})", run.created_at.format("%d/%m/%Y %H:%M")),
        download_url: GET_ARTIFACT_FOR_RUN.replace('%', &run.id.to_string()),
        url: run.url,
        changelog,
    }))
}

pub fn check_stable_release() -> anyhow::Result<Option<AvailableUpdate>> {
    #[derive(Deserialize)]
    struct ReleasePartial {
        pub tag_name: String,
        pub name: String,
        pub url: String,

        pub assets: Vec<AssetPartial>,
        pub body: String,
    }

    #[derive(Deserialize)]
    struct AssetPartial {
        pub name: String,
        pub browser_download_url: String,
    }

    let response = github_get(GET_RELEASES.replace('%', REPOSITORY))?.error_for_status()?;
    let releases: Vec<ReleasePartial> = response.json()?;

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
        url: release.url,
        changelog: release.body,
    }))
}
