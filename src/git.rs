use git2::{Commit, ObjectType, Oid, Repository, Signature, Sort};
use git2;
use std::error::Error;
use std::path::Path;
use git2::RemoteCallbacks;
use git2::PushOptions;

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
    walker.nth(1).unwrap().unwrap()
}

pub fn push_sha<S: AsRef<str>>(repo: &Repository, sha: Oid, ref_name: S) -> Result<(), Box<Error>> {
    println!("Remote: {}", ref_name.as_ref());
    let mut callbacks = RemoteCallbacks::new();
    callbacks.push_update_reference(|name, err| {
        println!("{:?}", err);
        Ok(())
    });

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks);
    let mut remote = repo.find_remote("origin").unwrap();
    repo.set_head_detached(sha).unwrap();

    let refspec_str = format!("HEAD:{}", ref_name.as_ref());
    let refspec_ref = refspec_str.as_str();
    println!("{}", refspec_str);

    let parts: Vec<&str> = vec![refspec_ref];
    remote.push(&parts, Some(&mut push_opts)).unwrap();

    Ok(())
}

pub fn commit_empty(
    repo: &Repository,
    author: &Signature,
    committer: &Signature,
    message: &str,
    parents: &[&Commit],
) -> Result<Oid, Box<Error>> {
    let new_empty_index_oid = repo.index()?.write_tree()?;
    let new_empty_object = repo.find_object(new_empty_index_oid, Some(ObjectType::Tree))?;
    let new_empty_tree = new_empty_object.as_tree().unwrap();

    Ok(repo.commit(
        Some("HEAD"),
        &author,
        &committer,
        &message,
        new_empty_tree,
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

pub fn is_not_tag<S: AsRef<str>>(value: &S) -> bool {
    //value.as_ref().starts_with("refs/heads") || value.as_ref() == "HEAD"
    !value.as_ref().starts_with("refs/tags")
}
