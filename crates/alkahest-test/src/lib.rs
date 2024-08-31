mod maps;

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use alkahest_pm::PACKAGE_MANAGER;
use alkahest_renderer::{
    gpu::GpuContext,
    renderer::{Renderer, RendererShared},
};
use anyhow::Context;
use clap::Parser;
use destiny_pkg::{GameVersion, PackageManager};
use mimalloc::MiMalloc;
use tracing_subscriber::{
    filter::filter_fn, fmt::Subscriber, layer::SubscriberExt, util::SubscriberInitExt,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Debug, Clone)]
#[command(about, long_about = None, disable_version_flag(true))]
struct TestArgs {
    /// Packages directory
    package_dir: Option<String>,
}

fn initialize_package_manager(/* args: &TestArgs*/) -> anyhow::Result<()> {
    let package_dir = /*if let Some(p) = &args.package_dir {
        if p.ends_with(".pkg") {
            PathBuf::from_str(p)
                .context("Invalid package directory")?
                .parent()
                .unwrap()
                .to_path_buf()
        } else {
            PathBuf::from_str(p).context("Invalid package directory")?
        }
    } else */ if let Some(package_dir_env) = std::env::var_os("ALKTEST_PACKAGES_DIR") {
        PathBuf::from_str(package_dir_env.to_str().unwrap()).context("Invalid package directory")?
    } else {
        panic!("No package directory specified!")
    };

    if !package_dir.exists() {
        panic!(
            "The specified package directory does not exist! ({})\nRelaunch alkahest-test with a \
             valid package directory.",
            package_dir.display()
        );
    }

    let pm = PackageManager::new(package_dir, GameVersion::Destiny2TheFinalShape).unwrap();

    *PACKAGE_MANAGER.write() = Some(Arc::new(pm));

    Ok(())
}

pub struct TestHarness {
    /// Headless renderer
    pub renderer: RendererShared,
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl TestHarness {
    pub fn new() -> Self {
        // Using try_init() instead of init() to avoid panicking if the logger is already initialized by another test thread
        // tracing_subscriber::fmt::try_init().ok();

        let builder = Subscriber::builder()
            .compact()
            .without_time()
            .with_thread_ids(true);
        builder.finish()
            // Filter anything but the info level
            .with(filter_fn(|metadata| matches!(*metadata.level(), tracing::Level::INFO | tracing::Level::ERROR)))
            .try_init().ok();

        initialize_package_manager(/*&TestArgs::parse()*/)
            .expect("Failed to initialize package manager");
        let gpu =
            Arc::new(GpuContext::create_headless().expect("Failed to create headless GPU context"));
        let renderer =
            Renderer::create(gpu, (4, 4), true).expect("Failed to create headless renderer");

        Self { renderer }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness() {
        let _harness = TestHarness::new();
    }
}
