use clap::Parser;

pub const ALKAHEST_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
pub struct AppArgs {
    /// Game directory
    #[arg(short, long)]
    pub gamedir: Option<String>,

    /// What display the window should be on
    #[arg(long)]
    pub display: Option<usize>,

    #[arg(long)]
    pub open_map: Option<String>,

    #[arg(long)]
    pub test_scene: bool,
}
