use git2::{Repository, Oid, Commit, Sort, RemoteCallbacks, PushOptions};
use std::path::{Path, PathBuf};
use std;
use std::fs;
use Error;
use git2;

mod fs_util;
mod git;
mod map;
mod copier;

pub struct WrappedSubGit {
    pub location: PathBuf,
    pub map: Repository,
    pub upstream_working: Repository,
    pub upstream_bare: Repository,
    pub local_working: Repository,
    pub local_bare: Repository,
    pub subdir: String
}

fn reverse_topological() -> Sort {
    let mut bits = 0 as u32;
    bits |= Sort::TOPOLOGICAL.bits();
    bits |= Sort::REVERSE.bits();
    Sort::from_bits(bits).unwrap()
}

fn reverse_topological_time() -> Sort {
    let mut bits = 0 as u32;
    bits |= Sort::TOPOLOGICAL.bits();
    bits |= Sort::REVERSE.bits();
    bits |= Sort::TIME.bits();
    Sort::from_bits(bits).unwrap()
}

fn find_earliest_commit(repo: &Repository ) -> Oid {
    let walker = &mut repo.revwalk().unwrap();
    walker.push(repo.find_reference("HEAD").unwrap().peel_to_commit().unwrap().id()).unwrap();
    walker.set_sorting(reverse_topological_time());
    walker.nth(1).unwrap().unwrap()
}

impl WrappedSubGit {
    pub fn open<SP: AsRef<Path>>(subgit_location: SP) -> Result<WrappedSubGit, Box<Error>> {
        let subgit_top_path : &Path = subgit_location.as_ref();
        let subgit_path = subgit_top_path.join("data");
        Ok(WrappedSubGit {
            location: subgit_path.to_owned(),
            map: Repository::open(subgit_path.join("map"))?,
            upstream_working: Repository::open(subgit_path.join("upstream"))?,
            upstream_bare: Repository::open(subgit_path.join("upstream.git"))?,
            local_working: Repository::open(subgit_path.join("local"))?,
            local_bare: Repository::open(subgit_path.join("local.git"))?,
            subdir: String::new()
        })
    }

    fn get_commits_to_import(&self, old_upstream_sha : Option<Oid>, new_upstream_sha: &Oid) -> Vec<Oid> {
        let walker = &mut self.upstream_bare.revwalk().unwrap();
        if let Some(old_sha) = old_upstream_sha {
            walker.hide(old_sha).unwrap();
        }
        walker.push(*new_upstream_sha).unwrap();
        walker.set_sorting(reverse_topological());
        let res : Result<Vec<Oid>,_> = walker.collect();
        return res.unwrap()
    }

    fn import_upstream_commits(&self, ref_name: &str, old_upstream_sha : Option<Oid>, new_upstream_sha: &Oid) {
        if Some(*new_upstream_sha) == old_upstream_sha { return }

        let mapper = map::CommitMapper { map: &self.map };

        let commits = self.get_commits_to_import(old_upstream_sha, new_upstream_sha);

        let sha_copier = copier::Copier {
            source: copier::GitLocation {
                name: "upstream",
                bare: &self.upstream_bare,
                working: &self.upstream_working,
                location: &self.subdir.as_str().as_ref(),
            },
            dest: copier::GitLocation {
                name: "local",
                bare: &self.local_bare,
                working: &self.local_working,
                location: "new".as_ref(),
            },
            mapper: &mapper
        };

        let maybe_new_sha : Option<Oid> = commits.into_iter()
            .filter(|&oid| !mapper.has_sha(&oid, "upstream", "local"))
            //.take(20)
            .map(|oid| sha_copier.copy_commit(&oid))
            .last();

        let new_sha = sha_copier.get_dest_sha(new_upstream_sha);


        let branch = sha_copier.dest.working.branch("test", &sha_copier.dest.working.find_commit(new_sha).unwrap(), true).unwrap();
        println!("Remote: {}", ref_name);
        let mut callbacks = RemoteCallbacks::new();
        callbacks.push_update_reference(|name, err| {
            println!("{:?}", err);
            Ok(())
        });
        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);
        let mut remote = sha_copier.dest.working.find_remote("origin").unwrap();
        sha_copier.dest.working.set_head("refs/heads/test").unwrap();

        let refspec_str = format!("refs/heads/test:{}", ref_name);
        let refspec_ref = refspec_str.as_str();
        println!("{}", refspec_str);
        let parts : Vec<&str> = vec!(refspec_ref);
        remote.push(&parts, Some(&mut push_opts)).unwrap();
    }

    pub fn update_all_from_upstream(&self) -> Result<(), Box<Error>>{
        let mut local_refs : std::collections::HashMap<String, git2::Oid> =
            git::get_refs(&self.upstream_bare, "**")?.into_iter()
                .filter(|&(ref name, ref _target)| git::is_standard(&name))
                .collect();

        let mapper = map::CommitMapper {map: &self.map};

        git::get_refs(&self.upstream_bare, "**")?.into_iter()
            .filter(|&(ref name, ref _target)| git::is_standard(&name))
            .for_each(|(ref_name, upstream_sha)| {
                info!("Importing {}", ref_name);
                let local_sha = local_refs.remove(&ref_name);
                let old_upstream_sha = mapper.get_translated(local_sha, "upstream", "local");

                if old_upstream_sha != Some(upstream_sha) {
                    &self.import_upstream_commits(&ref_name, old_upstream_sha, &upstream_sha);
                }
            });

        // TODO: iterate over the leftover keys

        Ok(())
    }

    pub fn create_or_fail<SP: AsRef<Path>, UP: AsRef<Path>> (subgit_location: SP, upstream_location: UP, subdir_loc: &str) -> Result<WrappedSubGit, Box<Error>> {
        let subgit_path : &Path = subgit_location.as_ref();
        let upstream_path : &Path = upstream_location.as_ref();
        let subgit_data_path = subgit_path.join("data");

        Repository::open_bare(&upstream_path)?;
        Repository::init_bare(&subgit_path)?;

        info!("Creating the mapping repo");
        let map = Repository::init(subgit_data_path.join("map"))?;

        info!("Creating upstream access (symlinking)");
        let upstream_path_abs = fs_util::make_absolute(upstream_path)?;
        std::os::unix::fs::symlink(&upstream_path_abs, subgit_data_path.join("upstream.git"))?;


        info!("Creating upstream working directory (for moving changes from subdir -> upstream)");
        let upstream_working = Repository::clone(&upstream_path_abs.to_string_lossy(), subgit_data_path.join("upstream"))?;


        info!("Creating mirror bare access (using symlinks, but excluding hooks)");
        let mirror_raw_path = subgit_data_path.join("local.git");
        fs::create_dir(&mirror_raw_path)?;
        // Symlink most directorys
        fs_util::symlink_dirs(&subgit_path, &mirror_raw_path, &vec!("config","description","info","logs","objects","refs","packed-refs"))?;
        // Copy HEAD (git doesn't like a HEAD that's a symlink)
        fs::copy(subgit_path.join("HEAD"), mirror_raw_path.join("HEAD"))?;
        // And we don't want to copy the hooks
        fs::create_dir(mirror_raw_path.join("hooks"))?;


        info!("Create mirror working directory (for moving changes from upstream -> subdir)");
        let mirror_working = Repository::clone(&subgit_path.to_string_lossy(), subgit_data_path.join("local"))?;

        info!("Adding general purpose empty commit in mirror working directory");
        let upstream_bare = Repository::open_bare(subgit_data_path.join("upstream.git"))?;
        {
            let earliest_commit =  upstream_bare.find_commit(find_earliest_commit(&upstream_bare))?;
            let new_empty_base_commit = git::commit_empty(&mirror_working, &earliest_commit.author(), &earliest_commit.committer(), "Imported empty commit", &vec!())?;
            mirror_working.reference("refs/sync/empty", new_empty_base_commit, false, "")?;
        }


        Ok(WrappedSubGit {
            location: subgit_location.as_ref().to_owned(),
            map: map,
            upstream_working: upstream_working,
            upstream_bare: upstream_bare,
            local_working: mirror_working,
            local_bare: Repository::open_bare(subgit_data_path.join("local.git"))?,
            subdir: subdir_loc.to_owned(),
        })
    }
}