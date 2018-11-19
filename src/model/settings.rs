use crate::action::RecursionDetection;
use crate::action::RecursionStatus;
use crate::fs;
use crate::logging;
use log::LevelFilter;
use log_panics;
use serde_json;
use std::path::{Path, PathBuf};

pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Serialize, Deserialize, Debug)]
struct SettingsFile {
    upstream_path: String,
    subgit_path: String,
    file_log_level: LevelFilter,
    recursion_detection: RecursionDetection,
    filters: Vec<String>,
}

pub struct Settings {
    internal: SettingsFile,
    data_dir: PathBuf,
}

impl Settings {
    pub fn generate<P: AsRef<Path>>(
        path: P,
        upstream_path: String,
        subgit_path: String,
        file_log_level: LevelFilter,
        recursion_detection: RecursionDetection,
        filters: Vec<String>,
    ) {
        let data_dir = path.as_ref();
        fs::write_content_to_file(
            &data_dir.join(SETTINGS_FILE),
            &serde_json::to_string_pretty(&SettingsFile {
                upstream_path,
                subgit_path,
                file_log_level,
                recursion_detection,
                filters,
            })
            .unwrap(),
        );
    }

    pub fn should_abort_hook(&self) -> bool {
        let status: RecursionStatus = self.recursion_detection().detect_recursion();
        let status_str = if status.is_recursing {
            "Detected hook recursion"
        } else {
            "No hook recursion detected"
        };
        info!("{} - {}", status_str, status.reason);
        status.is_recursing
    }

    pub fn recursion_detection(&self) -> RecursionDetection {
        self.internal.recursion_detection.clone()
    }

    pub fn upstream_path(&self) -> String {
        self.internal.upstream_path.clone()
    }

    pub fn local_path(&self) -> String {
        self.internal.subgit_path.clone()
    }

    pub fn filters(&self) -> Vec<String> {
        self.internal.filters.clone()
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Settings {
        let data_dir = path.as_ref();
        let contents = fs::content_of_file_if_exists(&path.as_ref().join(SETTINGS_FILE)).unwrap();
        Settings {
            internal: serde_json::from_str(contents.as_str()).unwrap(),
            data_dir: data_dir.to_owned(),
        }
    }

    pub fn setup_logging(&self) {
        logging::configure_logging(
            LevelFilter::Warn,
            self.internal.file_log_level,
            &self.data_dir.join("logs").join("sync.log"),
        );
        log_panics::init();
    }
}
