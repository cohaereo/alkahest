use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
pub struct AppArgs {
    /// Game directory
    #[arg(short, long)]
    pub gamedir: Option<String>,

    /// What display the window should be on
    #[arg(long)]
    pub display: Option<usize>,
}
