use anyhow::Context;
use serde::Deserialize;

use crate::cli::ALKAHEST_VERSION;

#[derive(Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub download_url: String,
    pub url: String,
    pub changelog: String,
}

const REPOSITORY: &str = "cohaereo/alkahest";

const API_ENDPOINT: &str = "https://api.github.com";
const GET_RELEASES: &str = "/repos/%/releases";

fn github_get<P: AsRef<str>>(path: P) -> ehttp::Request {
    let url = format!("{}{}", API_ENDPOINT, path.as_ref());
    ehttp::Request::get(url)
        .with_header("User-Agent", "alkahest")
        .with_header("Accept", "application/vnd.github.v3+json")
        .with_header("X-GitHub-Api-Version", "2022-11-28")
        .with_timeout(Some(std::time::Duration::from_secs(10)))
}

pub fn check_stable_release() -> anyhow::Result<Option<AvailableUpdate>> {
    #[derive(Deserialize, Debug)]
    struct ReleasePartial {
        pub tag_name: String,
        pub name: String,
        pub html_url: String,

        pub assets: Vec<AssetPartial>,
        pub body: String,
    }

    #[derive(Deserialize, Debug)]
    struct AssetPartial {
        pub name: String,
        pub browser_download_url: String,
    }

    let request = github_get(GET_RELEASES.replace('%', REPOSITORY));
    let releases: Vec<ReleasePartial> = match ehttp::fetch_blocking(&request) {
        Ok(response) => response.json()?,
        Err(e) => {
            anyhow::bail!("Failed to fetch releases: {e}")
        }
    };

    let release = releases.into_iter().next().context("No releases found")?;
    let release_semver = semver::Version::parse(&version_fixup(&release.tag_name))?;
    let current_semver = semver::Version::parse(&version_fixup(ALKAHEST_VERSION))?;

    if release_semver <= current_semver {
        info!("No updates found, v{} is up to date!", ALKAHEST_VERSION);
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

/// Fixes version/tag strings to be compatible with semver
pub fn version_fixup(version: &str) -> String {
    let v = version.replace('v', "");
    if v.chars().filter(|c| *c == '.').count() == 1 {
        format!("{}.0", v)
    } else {
        v
    }
}
