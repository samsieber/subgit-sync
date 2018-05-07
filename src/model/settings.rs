use toml;
use serde;
use log::LevelFilter;
use std::path::{PathBuf, Path};
use std::fs::File;
use logging;
use fs;

#[derive(Serialize, Deserialize, Debug)]
struct SettingsFile {
    upstream_path: String,
    subgit_path: String,
    file_log_level: LevelFilter,
}

pub struct Settings {
    internal: SettingsFile,
    data_dir: PathBuf,
}

impl Settings {
    pub fn generate<P: AsRef<Path>>(path : P, upstream_path: String, subgit_path: String, file_log_level: LevelFilter) {
        let data_dir = path.as_ref();
        fs::write_content_to_file(&data_dir.join("settings.toml"), &toml::to_string(
            &SettingsFile {
                upstream_path: upstream_path,
                subgit_path: subgit_path,
                file_log_level: file_log_level,
            }
        ).unwrap());
    }

    pub fn upstream_path(&self) -> String {
        self.internal.upstream_path.clone()
    }

    pub fn local_path(&self) -> String {
        self.internal.subgit_path.clone()
    }

    pub fn load<P: AsRef<Path>>(path : P) -> Settings {
        let data_dir = path.as_ref();
        let contents = fs::content_of_file_if_exists(&path.as_ref().join("settings.toml")).unwrap();
        Settings {
            internal: toml::from_str(contents.as_str()).unwrap(),
            data_dir: data_dir.to_owned()
        }
    }

    pub fn setup_logging(&self){
        logging::configure_logging(LevelFilter::Warn, self.internal.file_log_level, &self.data_dir.join("log").join("file.log"));
    }
}
