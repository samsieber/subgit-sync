use git2::{Oid, PushOptions, RemoteCallbacks, Repository, Sort};
use std::path::{Path, PathBuf};
use std;
use std::error::Error;
use git2;

use fs;
use git;
use util;

mod map;
mod copier;
mod settings;

use log::LevelFilter;

pub struct WrappedSubGit {
    pub location: PathBuf,
    pub map: Repository,
    pub upstream_working: Repository,
    pub upstream_bare: Repository,
    pub upstream_path: String,
    pub local_working: Repository,
    pub local_bare: Repository,
    pub local_path: String,
}

impl WrappedSubGit {
    pub fn open<SP: AsRef<Path>>(subgit_location: SP) -> Result<WrappedSubGit, Box<Error>> {
        let subgit_top_path: &Path = subgit_location.as_ref();
        let subgit_path = subgit_top_path.join("data");
        let git_settings = settings::Settings::load(&subgit_path);

        git_settings.setup_logging();

        Ok(WrappedSubGit {
            location: subgit_path.to_owned(),
            map: Repository::open(subgit_path.join("map"))?,
            upstream_working: Repository::open(subgit_path.join("upstream"))?,
            upstream_bare: Repository::open(subgit_path.join("upstream.git"))?,
            upstream_path: git_settings.upstream_path(),
            local_working: Repository::open(subgit_path.join("local"))?,
            local_bare: Repository::open(subgit_path.join("local.git"))?,
            local_path: git_settings.local_path(),
        })
    }

    pub fn push_ref_change_upstream<S: AsRef<str>>(
        &self,
        ref_name: S,
        old_sha: Oid,
        new_sha: Oid,
    ) -> Result<(), Box<Error>> {
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

        if new == None {
            self.local_bare.reflog_delete(ref_name.as_ref())?;
            return Ok(());
        }

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

        if old_upstream != real_upstream && real_upstream != None {
            let new_old_local_sha = self.import_upstream_commits(
                ref_name.as_ref(),
                old_upstream,
                &real_upstream.unwrap(),
            );
            if old != new_old_local_sha {
                return Err(Box::new(util::StringError {
                    message: "Out of sync with the upstream repo!".to_owned(),
                }));
            }
        }

        self.export_local_commits(ref_name.as_ref(), old, &new_sha);

        Ok(())
    }


    fn export_local_commits(
        &self,
        ref_name: &str,
        old_local_sha: Option<Oid>,
        new_local_sha: &Oid,
    ) -> Option<Oid> {
        let mapper = map::CommitMapper { map: &self.map };
        let sha_copier = copier::Copier {
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
            mapper: &mapper,
        };

        sha_copier.copy_ref_unchecked(ref_name, old_local_sha, new_local_sha)
    }

    fn import_upstream_commits(
        &self,
        ref_name: &str,
        old_upstream_sha: Option<Oid>,
        new_upstream_sha: &Oid,
    ) -> Option<Oid> {
        let mapper = map::CommitMapper { map: &self.map };
        let sha_copier = copier::Copier {
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
            mapper: &mapper,
        };

        sha_copier.copy_ref_unchecked(ref_name, old_upstream_sha, new_upstream_sha)
    }

    pub fn update_all_from_upstream(&self) -> Result<(), Box<Error>> {
        let mut local_refs: std::collections::HashMap<String, git2::Oid> =
            git::get_refs(&self.local_bare, "**")?
                .into_iter()
                .filter(|&(ref name, ref _target)| git::is_not_tag(&name))
                .collect();

        let mapper = map::CommitMapper { map: &self.map };

        git::get_refs(&self.upstream_bare, "**")?
            .into_iter()
            .filter(|&(ref name, ref _target)| git::is_not_tag(&name))
            .for_each(|(ref_name, upstream_sha)| {
                info!("Importing {}", ref_name);
                let local_sha = local_refs.remove(&ref_name);
                info!(
                    "Importing {} to point to {} (Was {:?} in the local)",
                    ref_name, upstream_sha, local_sha
                );
                let old_upstream_sha = mapper.get_translated(local_sha, "upstream", "local");

                &self.import_upstream_commits(&ref_name, old_upstream_sha, &upstream_sha);
            });

        // TODO: iterate over the leftover keys

        Ok(())
    }

    pub fn create_or_fail<SP: AsRef<Path>, UP: AsRef<Path>>(
        subgit_location: SP,
        upstream_location: UP,
        subdir_loc: &str,
    ) -> Result<WrappedSubGit, Box<Error>> {
        WrappedSubGit::run_creation(
            subgit_location,
            upstream_location,
            subdir_loc,
            None,
            LevelFilter::Debug,
        )
    }

    pub fn run_creation<SP: AsRef<Path>, UP: AsRef<Path>>(
        subgit_location: SP,
        upstream_location: UP,
        upstream_map_path: &str,
        subgit_map_path: Option<&str>,
        log_level: LevelFilter,
    ) -> Result<WrappedSubGit, Box<Error>> {
        let subgit_path: &Path = subgit_location.as_ref();
        let upstream_path: &Path = upstream_location.as_ref();
        let subgit_data_path = subgit_path.join("data");

        Repository::open_bare(&upstream_path)?;
        Repository::init_bare(&subgit_path)?;

        info!("Creating the mapping repo");
        let map = Repository::init(subgit_data_path.join("map"))?;

        info!("Creating upstream access (symlinking)");
        let upstream_path_abs = fs::make_absolute(upstream_path)?;
        std::os::unix::fs::symlink(&upstream_path_abs, subgit_data_path.join("upstream.git"))?;

        info!("Creating upstream working directory (for moving changes from subdir -> upstream)");
        let upstream_working = Repository::clone(
            &upstream_path_abs.to_string_lossy(),
            subgit_data_path.join("upstream"),
        )?;

        info!("Creating mirror bare access (using symlinks, but excluding hooks)");
        let mirror_raw_path = subgit_data_path.join("local.git");
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
        let mirror_working = Repository::clone(
            &subgit_path.to_string_lossy(),
            subgit_data_path.join("local"),
        )?;

        info!("Adding general purpose empty commit in mirror working directory");
        let upstream_bare = Repository::open_bare(subgit_data_path.join("upstream.git"))?;
        {
            let earliest_commit =
                upstream_bare.find_commit(git::find_earliest_commit(&upstream_bare))?;
            let new_empty_base_commit = git::commit_empty(
                &mirror_working,
                &earliest_commit.author(),
                &earliest_commit.committer(),
                "Empty base commit - autogenerated",
                &vec![],
            )?;
            mirror_working.reference(
                "refs/sync/empty",
                new_empty_base_commit,
                false,
                "generating empty base commit",
            )?;
        }

        settings::Settings::generate(
            &subgit_data_path,
            upstream_map_path.to_string(),
            subgit_map_path.unwrap_or("").to_owned(),
            log_level,
        );

        Ok(WrappedSubGit {
            location: subgit_location.as_ref().to_owned(),
            map,
            upstream_working,
            upstream_bare,
            upstream_path: upstream_map_path.to_owned(),
            local_working: mirror_working,
            local_bare: Repository::open_bare(subgit_data_path.join("local.git"))?,
            local_path: subgit_map_path.unwrap_or("").to_owned(),
        })
    }
}
