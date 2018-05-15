use std::path::{PathBuf, Path};
use std::fs::{remove_dir_all, create_dir_all};
use std::error::Error;
use util;
use model::WrappedSubGit;
use log::LevelFilter;
use model::BinSource;
use simplelog::TermLogger;
use simplelog::Config;
use super::test_util::*;
use std::ops::Index;

pub fn run_basic_branch_test<P,K,V,F,I>(root: P, files_collection: I) -> Result<(), Box<Error>>
where P: AsRef<Path>, K: AsRef<Path>, V: AsRef<[u8]>, F: IntoIterator<Item=(K,V)>, I: IntoIterator<Item=F>
{
    let mut files = files_collection.into_iter();

    let _ = TermLogger::init(LevelFilter::Debug, Config::default());
//    let empty : Vec<String> = vec!();
    let d = root.as_ref();
    let up_bare = init_bare_repo("test.git", &d)?;
    let up = clone(&d, &up_bare)?;

    set_credentials(&up_bare);
    set_credentials(&up);

    {
        util::write_files(&up, files.next().unwrap())?;
        util::command(&up, "git", ["add", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "(1) First upstream commit"].iter())?;
        util::command(&up, "git", ["push"].iter())?;
    };
    let local_bare = init_bare_repo("local.git", &d)?;
    let wrapped = WrappedSubGit::run_creation(
        &local_bare,
        &up_bare,
        "sub",
        None,
        LevelFilter::Debug,
        BinSource {
            location: PathBuf::from("target/debug/hook"),
            symlink: true,
        },
        None,
        None,
    )?;

    wrapped.update_all_from_upstream()?;

    let local = clone(&d, &local_bare)?;

    set_credentials(&local_bare);
    set_credentials(&local);

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&local, files.next().unwrap())?;

        util::command(&local, "git", ["add", "."].iter())?;
        util::command(&local, "git", ["commit", "-m", "(2) First subgit commit"].iter())?;
        util::command(&local, "git", ["push"].iter())?;

        util::command(&up, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&up, files.next().unwrap())?;

        util::command(&up, "git", ["add", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "(3) Second upstream commit"].iter())?;
        util::command(&up, "git", ["push"].iter())?;


        util::command(&local, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    Ok(())
}