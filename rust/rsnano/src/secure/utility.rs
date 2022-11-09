use once_cell::sync::Lazy;
use std::{path::PathBuf, sync::Mutex};
use uuid::Uuid;

use crate::{config::NetworkConstants, core::Networks};

//todo refactor: this global state thing is not a good solution
static ALL_UNIQUE_PATHS: Lazy<Mutex<Vec<PathBuf>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub fn working_path() -> Option<PathBuf> {
    working_path_for(NetworkConstants::active_network())
}
pub fn working_path_for(network: Networks) -> Option<PathBuf> {
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
    unique_path_for(NetworkConstants::active_network())
}

pub fn unique_path_for(network: Networks) -> Option<PathBuf> {
    working_path_for(network).map(|mut path| {
        let uuid = Uuid::new_v4();
        path.push(uuid.to_string());
        ALL_UNIQUE_PATHS.lock().unwrap().push(path.clone());
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

        // lmdb creates a -lock suffixed file for its MDB_NOSUBDIR databases
        let mut lockfile = path.to_owned();
        let mut filename = lockfile.file_name().unwrap().to_os_string();
        filename.push("-lock");
        lockfile.set_file_name(filename);

        if std::fs::metadata(lockfile.as_path()).is_ok() {
            if let Err(e) = std::fs::remove_file(lockfile) {
                eprintln!("Could not remove temporary lock file: {}", e);
            }
        }
    }
    all.clear();
}
