use directories::ProjectDirs;
use lazy_static::lazy_static;

lazy_static! {
    static ref PORTABLE_DIR: std::path::PathBuf = {
        let exe_path = std::env::current_exe().expect("Failed to get current executable path");
        exe_path
            .parent()
            .expect("Failed to get executable directory")
            .to_owned()
    };
    static ref IS_PORTABLE: bool = {
        let portable_path = PORTABLE_DIR.join("portable.txt");
        let is_portable = portable_path.exists();
        if is_portable {
            tracing::info!("Running in portable mode");
            std::fs::create_dir_all(PORTABLE_DIR.join("config"))
                .expect("Failed to create portable config directory");
            std::fs::create_dir_all(PORTABLE_DIR.join("local"))
                .expect("Failed to create portable local config directory");
        }

        is_portable
    };
    static ref APP_DIRS: ProjectDirs = {
        let pd = ProjectDirs::from("net", "cohaereo", "Alkahest")
            .expect("Failed to get application directories");
        if !(*IS_PORTABLE) {
            std::fs::create_dir_all(pd.config_dir()).expect("Failed to create config directory");
            std::fs::create_dir_all(pd.config_local_dir())
                .expect("Failed to create local config directory");
        }

        pd
    };
}

pub fn config_dir() -> std::path::PathBuf {
    if *IS_PORTABLE {
        PORTABLE_DIR.join("config")
    } else {
        APP_DIRS.config_dir().to_owned()
    }
}

pub fn local_config_dir() -> std::path::PathBuf {
    if *IS_PORTABLE {
        PORTABLE_DIR.join("local")
    } else {
        APP_DIRS.config_local_dir().to_owned()
    }
}
