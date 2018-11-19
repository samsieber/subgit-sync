#[macro_use]
extern crate maplit;

extern crate subgit_rs;
extern crate log;
extern crate simplelog;

mod util;
mod harness;

use crate::util::*;
use crate::harness::*;

#[test]
fn modify_modify_modify() {
    let data = vec![
        hashmap!{ "sub/testing" => "This is another test. Yaya."},
        hashmap!{ "testing" => "Overwritten in local"},
        hashmap!{ "sub/testing" => "Overwritten in upstream" },
    ];

    run_basic_branch_test( &test_dir("modify_modify_modify"), data).unwrap();
}

#[test]
fn add_new_in_subgit_modify_upstream() {
    let data = vec![
        hashmap!{ "sub/testing.txt" => "This is another test. Yaya.", "sub/hello.txt" => "Hello world"},
        hashmap!{ "testing" => "New in local"},
        hashmap!{ "sub/testing" => "Overwritten in upstream", "sub/new.txt" => "Hello again" },
    ];

    run_basic_branch_test( &test_dir("add_new_in_subgit_modify_upstream"), data).unwrap();
}