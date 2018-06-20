use git2::{Commit, ObjectType, Oid, Repository, Signature, Sort};
use git2;
use std::error::Error;
use std::path::Path;
use std;
use util::StringError;

pub fn get_git_options() -> Option<Vec<String>> {
    std::env::var_os("GIT_PUSH_OPTION_COUNT").map(|v| {
        let git_opt_count = v.into_string().expect("GIT_PUSH_OPTION_COUNT is unreadable").parse::<u32>().expect("GIT_PUSH_OPTION_COUNT is supposed to be a env variable that's a number");

        (0..git_opt_count).map(|i|
            std::env::var(&format!("GIT_PUSH_OPTION_{}", i)).expect(&format!("GIT_PUSH_OPTION_{} was unreadable", i))
        ).collect::<Vec<String>>()
    })
}

pub fn optionify_sha(oid: Oid) -> Option<Oid> {
    if oid == no_sha() {
        None
    } else {
        Some(oid)
    }
}

pub fn no_sha() -> Oid {
    Oid::from_str("0000000000000000000000000000000000000000").unwrap()
}

pub fn reverse_topological() -> Sort {
    let mut bits = 0 as u32;
    bits |= Sort::TOPOLOGICAL.bits();
    bits |= Sort::REVERSE.bits();
    Sort::from_bits(bits).unwrap()
}

pub fn reverse_topological_time() -> Sort {
    let mut bits = 0 as u32;
    bits |= Sort::TOPOLOGICAL.bits();
    bits |= Sort::REVERSE.bits();
    bits |= Sort::TIME.bits();
    Sort::from_bits(bits).unwrap()
}

pub fn open_or_clone_bare<P: AsRef<Path>>(path: P, url: &str) -> Repository {
    match Repository::open_bare(&path) {
        Ok(repo) => repo,
        Err(e) => {
            info!(
                "Couldn't open repo at {}, attempting clone from {}. Original error: {:?}",
                &path.as_ref().to_string_lossy(),
                &url,
                e
            );
            let mut builder = git2::build::RepoBuilder::new();
            match builder.bare(true).clone(url, path.as_ref()) {
                Ok(repo) => repo,
                Err(e) => panic!("failed to open or clone clone: {}", e),
            }
        }
    }
}

pub fn disable_gc(repo: &Repository) {
    let mut config = repo.config().unwrap();
    config.set_str("gc.pruneExpire", "never").unwrap();
    config.set_str("gc.reflogExpire", "never").unwrap();
    config.set_i32("gc.auto", 0).unwrap();
}

pub fn set_push_simple(repo: &Repository){
    let mut config = repo.config().unwrap();
    config.set_str("push.default", "simple").unwrap();
}

fn find_earliest_commit_on_folder(repo: &Repository, target: &str) -> Option<Oid> {
    //git rev-list --reverse --topo-order HEAD --
    let mut process = std::process::Command::new("git");
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());

    process.arg("rev-list");
    process.arg("--reverse");
    process.arg("--topo-order");
    process.arg("HEAD");
    process.arg("--");
    process.arg(target);

    process.current_dir(repo.path());

    debug!("Finding earliest commit in  {:?} / {}", repo.path(), target);

    let result = process.output().unwrap();

    if !result.status.success() {
        panic!("Could not read from repo");
    } else {
        let full = std::str::from_utf8(&result.stdout).unwrap();
        let oids = full.trim();
        info!("OIDS text: '{}'", &oids);
        oids.split('\n').filter(|v| !v.is_empty()).nth(0).map(|v| Oid::from_str(v).unwrap())
    }
}

pub fn find_safe_empty_na_commits(repo: &Repository, target: &str) -> Vec<Oid> {
    let earliest = find_earliest_commit_on_folder(repo, target);
    let mut rev_walk = repo.revwalk().unwrap();
    if let Some(earliest_oid) = earliest {
        rev_walk.push(earliest_oid).unwrap();
    } else {
        rev_walk.push_head().unwrap();
    }
    let mut commits: Vec<Oid> = rev_walk.into_iter().map(|v| v.unwrap()).collect();
    commits.remove(0);
    commits
}

pub fn find_earliest_commit(repo: &Repository) -> Oid {
    let walker = &mut repo.revwalk().unwrap();
    walker
        .push(
            repo.find_reference("HEAD")
                .unwrap()
                .peel_to_commit()
                .unwrap()
                .id(),
        )
        .unwrap();
    walker.set_sorting(reverse_topological_time());
    walker.nth(0).unwrap().unwrap()
}

pub fn fetch_all_ext(repo: &Repository) -> Result<(), Box<Error>> {
    let mut process = std::process::Command::new("git");
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());
    process.arg("fetch");
    process.arg("--all");

    process.current_dir(repo.workdir().unwrap());

    debug!("Fetching all in {:?}", repo.workdir());

    let result = process.output()?;

    if !result.status.success() {
        return Err(Box::new(StringError { message: format!("Could not fetch all- exit code was {}. Full result of fetch: {}", &result.status, String::from_utf8(result.stderr)?) }));
    }

    Ok(())
}

pub fn get_n_recent_shas(repo: &Repository, n: usize) -> Vec<Oid> {
    //git for-each-ref --count=3 --sort='-authordate' --format='%(objectname) - %(subject)' refs/heads
    let mut process = std::process::Command::new("git");
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());
    process.arg("for-each-ref");
    process.arg(&format!("--count={}", n));
    process.arg("--sort=-authordate"); // Sort by author date, descending ( the '-' in author date means sort descending)
    process.arg("--format=%(objectname)"); // Output the sha (objectname)
    process.arg("refs/heads");

    process.current_dir(repo.path());

    debug!("Finding recent commits in {:?}", repo.path());

    let result = process.output().expect("Could not lookup recent commits");

    if !result.status.success() {
        panic!("Could not fetch all- exit code was {}. Full result of fetch: {}", &result.status, String::from_utf8(result.stderr).unwrap());
    } else {
        let full = std::str::from_utf8(&result.stdout).unwrap().trim();
//        let mut oids : Vec<Oid> =
            full.split('\n').filter(|v| !v.is_empty()).map(|v| Oid::from_str(v).unwrap()).collect()
//            ;
//        oids.push(repo.head().unwrap().peel_to_commit().unwrap().id());
//        oids
    }
}

pub fn clone_remote<S: AsRef<str>, P: AsRef<Path>>(url: S, parent: P, name: &str){
    std::process::Command::new("git")
        .arg("clone")
        .arg(url.as_ref())
        .arg(name)
        .current_dir(parent.as_ref())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

pub fn push_sha_ext<S: AsRef<str>>(repo: &Repository, ref_name: S, force_push: bool, git_push_options: Option<Vec<String>>) -> Result<(), Box<Error>> {
    let mut process = std::process::Command::new("git");
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());
    process.arg("push");

    if let Some(git_push_opts) = git_push_options {
        for val in git_push_opts {
            process.arg(format!("--push-option={}", val));
        }
    };

    process.arg("origin");
    if force_push {
        info!("Force pushing");
        process.arg(format!("+HEAD:{}", ref_name.as_ref()));
    } else {
        info!("Not force pushing");
        process.arg(format!("HEAD:{}", ref_name.as_ref()));
    }

    process.current_dir(repo.workdir().unwrap());

    info!("Pushing 'HEAD:{}' from {:?}", ref_name.as_ref(), repo.workdir());

    let result = process.output()?;

    if !result.status.success() {
        return Err(Box::new(StringError { message: format!("Could not push - exit code was {}. Full result of push: {}", &result.status, String::from_utf8(result.stderr)?) }));
    }

    Ok(())
}

pub fn delete_remote_branch<S: AsRef<str>>(repo: &Repository, ref_name: S, git_push_options: Option<Vec<String>>) -> Result<(), Box<Error>> {
    let mut process = std::process::Command::new("git");
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());
    if let Some(git_push_opts) = git_push_options {
        process.env("GIT_PUSH_OPTION_COUNT", format!("{}", git_push_opts.len()));
        for (idx, val) in git_push_opts.iter().enumerate() {
            process.env(format!("GIT_PUSH_OPTION_{}", idx), &val);
        }
    };
    process.arg("push");
    process.arg("origin");
    process.arg(format!(":{}", ref_name.as_ref()));

    process.current_dir(repo.workdir().unwrap());

    info!("Pushing ':{}' from {:?}", ref_name.as_ref(), repo.workdir());

    let result = process.output()?;

    if !result.status.success() {
        return Err(Box::new(StringError { message: format!("Could not push - exit code was {}. Full result of push: {}", &result.status, String::from_utf8(result.stderr)?) }));
    }

    Ok(())
}

pub fn commit_empty(
    repo: &Repository,
    ref_name: &str,
    author: &Signature,
    committer: &Signature,
    message: &str,
    parents: &[&Commit],
) -> Result<Oid, Box<Error>> {
    let new_empty_tree_oid = repo.treebuilder(None)?.write()?;
    let new_empty_tree = repo.find_tree(new_empty_tree_oid)?;

    Ok(repo.commit(
        Some(ref_name),
        &author,
        &committer,
        &message,
        &new_empty_tree,
        &parents,
    )?)
}

pub fn get_refs(repo: &Repository, glob: &str) -> Result<Vec<(String, Oid)>, Box<Error>> {
    let ref_list: Result<Vec<(String, Oid)>, _> = repo.references_glob(glob)?
        .map(|r| {
            let rr = r?;
            Ok((
                rr.name().expect("can't get ref name").to_owned(),
                rr.target().expect("Can't get target"),
            ))
        })
        .collect();
    ref_list
}

pub fn is_applicable<S: AsRef<str>>(value: &S) -> bool {
    value.as_ref().starts_with("refs/heads") || value.as_ref() == "HEAD"
//    !value.as_ref().starts_with("refs/tags")
}