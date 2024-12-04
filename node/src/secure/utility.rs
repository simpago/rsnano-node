use once_cell::sync::Lazy;
use rsnano_core::Networks;
use std::{path::PathBuf, sync::Mutex};
use uuid::Uuid;

use crate::config::NetworkConstants;

//todo refactor: this global state thing is not a good solution
static ALL_UNIQUE_PATHS: Lazy<Mutex<Vec<PathBuf>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn working_path() -> Option<PathBuf> {
    working_path_for(NetworkConstants::active_network())
}
pub fn working_path_for(network: Networks) -> Option<PathBuf> {
    if let Ok(path_override) = std::env::var("NANO_APP_PATH") {
        eprintln!(
            "Application path overridden by NANO_APP_PATH environment variable: {path_override}"
        );
        return Some(path_override.into());
    }

    dirs::home_dir().and_then(|mut path| {
        let subdir = match network {
            Networks::Invalid => return None,
            Networks::NanoDevNetwork => "NanoDev",
            Networks::NanoBetaNetwork => "NanoBeta",
            Networks::NanoLiveNetwork => "Nano",
            Networks::NanoTestNetwork => "NanoTest",
        };
        path.push(subdir);
        Some(path)
    })
}

pub fn unique_path() -> Option<PathBuf> {
    unique_path_for(Networks::NanoDevNetwork)
}

pub fn unique_path_for(network: Networks) -> Option<PathBuf> {
    working_path_for(network).map(|mut path| {
        let uuid = Uuid::new_v4();
        path.push(uuid.to_string());
        ALL_UNIQUE_PATHS.lock().unwrap().push(path.clone());
        std::fs::create_dir_all(&path).unwrap();
        path
    })
}

pub fn remove_temporary_directories() {
    let mut all = ALL_UNIQUE_PATHS.lock().unwrap();
    for path in all.iter() {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.is_file() {
                if let Err(e) = std::fs::remove_file(path) {
                    eprintln!("Could not remove temporary file '{:?}': {}", path, e);
                }
            } else if meta.is_dir() {
                if let Err(e) = std::fs::remove_dir_all(path) {
                    eprintln!("Could not remove temporary directory '{:?}': {}", path, e);
                }
            }
        }
    }
    all.clear();
}
