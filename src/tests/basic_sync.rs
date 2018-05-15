use std::path::{PathBuf, Path};
use std::fs::{remove_dir_all, create_dir_all};
use util;
use std::error::Error;
use fs;
use model::WrappedSubGit;
use log::LevelFilter;
use model::BinSource;
use simplelog::TermLogger;
use simplelog::Config;
use std;
use super::test_util::*;
use super::single_branch_basic::run_basic_branch_test;

fn run_basic() -> Res<()> {
    let data = vec![
        hashmap!{ "sub/testing" => "This is another test. Yaya."},
        hashmap!{ "testing" => "Overwritten in local"},
        hashmap!{ "sub/testing" => "Overwritten in upstream" },
        ];

    run_basic_branch_test( &test_dir("test_basic"), data)
}

fn run_basic_2() -> Res<()> {
    let data = vec![
        hashmap!{ "sub/testing.txt" => "This is another test. Yaya.", "sub/hello.txt" => "Hello world"},
        hashmap!{ "testing" => "New in local"},
        hashmap!{ "sub/testing" => "Overwritten in upstream", "sub/new.txt" => "Hello again" },
    ];

    run_basic_branch_test( &test_dir("test_basic_2"), data)
}
#[test]
fn test_push_from_local_then_pull_upstream_1(){
    assert_works(run_basic);
}

#[test]
fn test_push_from_local_then_pull_upstream_2(){
    assert_works(run_basic_2);
}