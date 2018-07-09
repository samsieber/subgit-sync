use git2::{Oid, Repository};
use std::path::{Path, PathBuf};
use std;
use std::error::Error;
use git2;

use fs;
use git;
use util;

mod map;
mod copier;
pub mod settings;

use log::LevelFilter;
use model::map::CommitMapper;
use simplelog::WriteLogger;
use simplelog::Config;
use std::fs::File;
use action::lock;
use action::RecursionDetection;
use action::RecursionStatus;

pub struct WrappedSubGit {
    pub location: PathBuf,
    pub map: Repository,
    pub upstream_working: Repository,
    pub upstream_bare: Repository,
    pub upstream_path: String,
    pub local_working: Repository,
    pub local_bare: Repository,
    pub local_path: String,
    pub recursion_detection: RecursionDetection,
    pub lock: File,
}

pub struct BinSource {
    pub location: PathBuf,
    pub symlink: bool,
}

impl WrappedSubGit {
    pub fn open<SP: AsRef<Path>, F: FnOnce()>(subgit_location: SP, before_load: Option<F>) -> Result<Option<WrappedSubGit>, Box<Error>> {
        let subgit_top_path: &Path = subgit_location.as_ref();
        let subgit_data_path = subgit_top_path.join("data");
        info!("Loading settings");
        let git_settings = settings::Settings::load(&subgit_data_path);
        info!("Loaded settings");

        git_settings.setup_logging();
        info!("Setup logging");

        if git_settings.should_abort_hook() {
            Ok(None)
        } else {
            if let Some(before_load_callback) = before_load {
                before_load_callback();
            }
            info!("Opened Wrapped");
            Ok(Some(WrappedSubGit {
                location: subgit_top_path.to_owned(),
                map: Repository::open(subgit_data_path.join("map")).expect("Cannot find map file"),
                upstream_working: Repository::open(subgit_data_path.join("upstream"))?,
                upstream_bare: Repository::open(subgit_data_path.join("upstream.git"))?,
                upstream_path: git_settings.upstream_path(),
                local_working: Repository::open(subgit_data_path.join("local"))?,
                local_bare: Repository::open(subgit_data_path.join("local.git"))?,
                local_path: git_settings.local_path(),
                recursion_detection: git_settings.recursion_detection(),
                lock: lock(&subgit_top_path).unwrap(),
            }))
        }
    }

    pub fn should_abort_hook(&self) -> bool {
        let status : RecursionStatus = self.recursion_detection.detect_recursion();
        let status_str = if status.is_recursing { "Detected hook recursion" } else { "No hook recursion detected" };
        info!("{} - {}", status_str, status.reason);
        status.is_recursing
    }

    pub fn update_self(&self) {
        git::fetch_all_ext(&self.local_working).unwrap();
        git::fetch_all_ext(&self.upstream_working).unwrap();
    }

    pub fn push_ref_change_upstream<S: AsRef<str>>(
        &self,
        ref_name: S,
        old_sha: Oid,
        new_sha: Oid,
    ) -> Result<(), Box<Error>> {
        info!("Starting on hook!");
        if !git::is_applicable(&ref_name.as_ref()) {
            info!("Skipping non-applicable ref: {}", ref_name.as_ref());
            return Ok(());
        }
        let old = if old_sha == git::no_sha() {
            None
        } else {
            Some(old_sha)
        };
        let new = if new_sha == git::no_sha() {
            None
        } else {
            Some(new_sha)
        };

        debug!("Post option adjustment");

        if new == None {
            //git::delete_remote_branch(&self.local_working, &ref_name, None)?;
            info!("Deleting remote branch");
            self.export_local_commits(ref_name.as_ref(), old, None);
            return Ok(());
        }

        info!("Updating ref: {} from {:?} -> {:?}", ref_name.as_ref(), old, new);
        let mapper = map::CommitMapper { map: &self.map };

        let old_upstream = mapper.get_translated(old, "local", "upstream");
        let real_upstream = self.upstream_bare
            .find_reference(ref_name.as_ref())
            .map(|reference| {
                Some(
                    reference
                        .target()
                        .expect("Reference is not direct - need to check for that"),
                )
            })
            .unwrap_or(None);

        info!("Found upstream commits");

        if old_upstream != real_upstream && real_upstream != None {
            info!("Importing new upstream commits first. Expected old upstream was {:?}, but real one is {:?}", old_upstream, real_upstream);
            let new_old_local_sha = self.import_upstream_commits(
                ref_name.as_ref(),
                old_upstream,
                real_upstream,
            );
            if old != new_old_local_sha {
                return Err(Box::new(util::StringError {
                    message: "Out of sync with the upstream repo!".to_owned(),
                }));
            }
        }

        info!("About to export commits");

        self.export_local_commits(&ref_name.as_ref(), old, Some(new_sha));

        println!("Exported commits from {} upstream", ref_name.as_ref());

        Ok(())
    }


    fn export_local_commits(
        &self,
        ref_name: &str,
        old_local_sha: Option<Oid>,
        new_local_sha: Option<Oid>,
    ) -> Option<Oid> {
        let sha_copier = self.get_exporter();

        sha_copier.copy_ref_unchecked(ref_name, old_local_sha, new_local_sha, false, self.recursion_detection.get_push_opts(), None::<&RecursionDetection>)
    }

    pub fn import_upstream_commits(
        &self,
        ref_name: &str,
        old_upstream_sha: Option<Oid>,
        new_upstream_sha: Option<Oid>,
    ) -> Option<Oid> {
        let sha_copier = self.get_importer();

        sha_copier.copy_ref_unchecked(ref_name, old_upstream_sha, new_upstream_sha, true, self.recursion_detection.get_push_opts(), Some(&self.recursion_detection))
    }

    fn get_importer<'a>(&'a self) -> copier::Copier<'a>{
        let mapper = map::CommitMapper { map: &self.map };
        copier::Copier {
            source: copier::GitLocation {
                name: "upstream",
                bare: &self.upstream_bare,
                working: &self.upstream_working,
                location: &self.upstream_path.as_str().as_ref(),
            },
            dest: copier::GitLocation {
                name: "local",
                bare: &self.local_bare,
                working: &self.local_working,
                location: &self.local_path.as_ref(),
            },
            mapper: mapper,
        }
    }

    fn get_exporter<'a>(&'a self) -> copier::Copier<'a>{
        let mapper = map::CommitMapper { map: &self.map };
        copier::Copier {
            dest: copier::GitLocation {
                name: "upstream",
                bare: &self.upstream_bare,
                working: &self.upstream_working,
                location: &self.upstream_path.as_str().as_ref(),
            },
            source: copier::GitLocation {
                name: "local",
                bare: &self.local_bare,
                working: &self.local_working,
                location: &self.local_path.as_ref(),
            },
            mapper: mapper,
        }
    }

    pub fn import_initial_empty_commits(&self) {
        let sha_copier = self.get_importer();

        sha_copier.import_initial_empty_commits();
    }

    pub fn update_all_from_upstream(&self) -> Result<(), Box<Error>> {
        let mut local_refs: std::collections::HashMap<String, git2::Oid> =
            git::get_refs(&self.local_bare, "**")?
                .into_iter()
                .filter(|&(ref name, ref _target)| git::is_applicable(&name))
                .collect();

        let mapper = map::CommitMapper { map: &self.map };

        git::get_refs(&self.upstream_bare, "**")?
            .into_iter()
            .filter(|&(ref name, ref _target)| git::is_applicable(&name))
            .for_each(|(ref_name, upstream_sha)| {
                info!("Importing {}", ref_name);
                let local_sha = local_refs.remove(&ref_name);
                info!(
                    "Importing {} to point to {} (Was {:?} in the local)",
                    ref_name, upstream_sha, local_sha
                );
                let old_upstream_sha = mapper.get_translated(local_sha, "upstream", "local");

                &self.import_upstream_commits(&ref_name, old_upstream_sha, Some(upstream_sha));
            });

        // TODO: iterate over the leftover keys

        Ok(())
    }

    pub fn run_creation<SP: AsRef<Path>, UP: AsRef<Path>>(
        subgit_location: SP,
        upstream_location: UP,
        upstream_map_path: &str,
        subgit_map_path: Option<&str>,
        log_level: LevelFilter,
        log_file: PathBuf,
        bin_loc: BinSource,
        subgit_hook_path: PathBuf,
        subgit_working_clone_url: Option<String>,
        upstream_hook_path: PathBuf,
        upstream_working_clone_url: Option<String>,
        recursion_detection: RecursionDetection,
    ) -> Result<WrappedSubGit, Box<Error>> {
        WriteLogger::init(
            LevelFilter::Debug,
            Config::default(),
            File::create(log_file).unwrap(),
        ).expect("Could not setup logging");

        let subgit_path: &Path = subgit_location.as_ref();
        let upstream_path: &Path = upstream_location.as_ref();
        let subgit_data_path = subgit_path.join("data");

        fs::create_dir_all(&subgit_data_path).unwrap();

        Repository::open_bare(&upstream_path)?;
        Repository::init_bare(&subgit_path)?;

        info!("Creating the logging directory");
        fs::create_dir_all(subgit_data_path.join("logs"))?;

        info!("Creating the mapping repo");
        let map = Repository::init(subgit_data_path.join("map"))?;

        info!("Creating upstream access (symlinking)");
        let upstream_path_abs = fs::make_absolute(upstream_path)?;
        std::os::unix::fs::symlink(&upstream_path_abs, subgit_data_path.join("upstream.git"))?;
//        let upstream_bare = Repository::open_bare(subgit_data_path.join("upstream.git"))?;

        info!("Creating upstream working directory (for moving changes from subdir -> upstream)");
        let upstream_url_to_clone = upstream_working_clone_url.unwrap_or_else(|| {
            upstream_path_abs.to_string_lossy().to_string()
        });
        git::clone_remote(
            &upstream_url_to_clone,
            &subgit_data_path,
            "upstream",
        );
        let upstream_working = Repository::open(subgit_data_path.join("upstream")).unwrap();
        git::disable_gc(&upstream_working);
        git::set_push_simple(&upstream_working);

        info!("Creating mirror bare access (using symlinks, but excluding hooks)");
        let mirror_raw_path = fs::make_absolute(subgit_data_path.join("local.git")).unwrap();
        fs::create_dir(&mirror_raw_path)?;
        // Symlink most directorys
        fs::symlink_dirs(
            &subgit_path,
            &mirror_raw_path,
            &vec![
                "config",
                "description",
                "info",
                "logs",
                "objects",
                "refs",
                "packed-refs",
            ],
        )?;
        // Copy HEAD (git doesn't like a HEAD that's a symlink)
        fs::copy(subgit_path.join("HEAD"), mirror_raw_path.join("HEAD"))?;
        // And we don't want to copy the hooks
        fs::create_dir(mirror_raw_path.join("hooks"))?;

        info!("Create mirror working directory (for moving changes from upstream -> subdir)");
        let subgit_url_to_clone = subgit_working_clone_url.unwrap_or_else(|| {
            mirror_raw_path.to_string_lossy().to_string()
        });
        git::clone_remote(
            &subgit_url_to_clone,
            &subgit_data_path,
            "local"
        );
        let mirror_working =  Repository::open(
            subgit_data_path.join("local"),
        )?;
        git::disable_gc(&mirror_working);
        git::set_push_simple(&mirror_working);

        info!("Adding general purpose empty commit in mirror working directory and upstream working directory");
        let upstream_bare = Repository::open_bare(subgit_data_path.join("upstream.git"))?;
        {
            let earliest_commit =
                upstream_bare.find_commit(git::find_earliest_commit(&upstream_bare))?;
            debug!("Found earliest commit!");

            let subgit_empty_sha = git::commit_empty(
                &mirror_working,
                "refs/sync/empty",
                &earliest_commit.author(),
                &earliest_commit.committer(),
                "Empty base commit - autogenerated",
                &vec![],
            )?;
            mirror_working.set_head("refs/sync/empty")?;
            git::push_sha_ext(&mirror_working, "refs/sync/empty", false, None)?;
//
//            if let Ok(reference) = upstream_bare.find_reference("refs/sync/empty") {
//                git::delete_remote_branch(upstream_working, "refs/sync/empty", None)?;
//            }

            let upstream_empty_sha = git::commit_empty(
                &upstream_working,
                "refs/sync/empty",
                &earliest_commit.author(),
                &earliest_commit.committer(),
                "Empty base commit - autogenerated",
                &vec![],
            )?;
            upstream_working.set_head("refs/sync/empty")?;
            git::push_sha_ext(&upstream_working, "refs/sync/empty", false, None)?;
            info!("Created {} as the empty upstream ref", &upstream_empty_sha);

            let mapper = CommitMapper { map: &map};

            mapper.set_translated(&upstream_empty_sha,"upstream", "local", &subgit_empty_sha);
            mapper.set_translated(&subgit_empty_sha, "local", "upstream", &upstream_empty_sha);
        }

        info!("Generating settings file");
        settings::Settings::generate(
            &subgit_data_path,
            upstream_map_path.to_string(),
            subgit_map_path.unwrap_or("").to_owned(),
            log_level,
            recursion_detection.clone(),
        );

        info!("Generating whitelist directory");
        fs::create_dir_all(&subgit_data_path.join("whitelist")).expect("Could not create whitelist folder");

        info!("Generating lock file");
        { File::create(&subgit_data_path.join("lock"))?; }
        info!("Preparing to lock");
        let lock = lock(&subgit_location)?;

        info!("Copying hook file");
        let hook_path = subgit_location.as_ref().join("data").join("hook");
        match bin_loc {
            BinSource {location, symlink: true } => std::os::unix::fs::symlink(fs::make_absolute(location)?, &hook_path)?,
            BinSource {location, symlink: false } => { std::fs::copy(location, &hook_path)?; },
        };

        info!("Adding subgit hook");
        std::os::unix::fs::symlink(fs::make_absolute(&hook_path)?, subgit_location.as_ref().join(subgit_hook_path))?;

        info!("Adding upstream hook");
        std::os::unix::fs::symlink(fs::make_absolute(&hook_path)?, upstream_location.as_ref().join(upstream_hook_path))?;

        Ok(WrappedSubGit {
            location: subgit_location.as_ref().to_owned(),
            map,
            upstream_working,
            upstream_bare,
            upstream_path: upstream_map_path.to_owned(),
            local_working: mirror_working,
            local_bare: Repository::open_bare(subgit_data_path.join("local.git"))?,
            local_path: subgit_map_path.unwrap_or("").to_owned(),
            recursion_detection,
            lock,
        })
    }
}
