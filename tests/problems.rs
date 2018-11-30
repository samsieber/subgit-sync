extern crate log;
extern crate simplelog;
extern crate subgit_rs;

mod harness;
mod util;

use crate::harness::*;
use std::time::Duration;

fn base(name: &str) -> TestWrapper {
    TestWrapper::new(
        name,
        |upstream| {
            upstream.update_working(vec![
                FileAction::overwrite(
                    "sub/hello.txt",
                    "Hello world (from upstream)\n",
                ),
                FileAction::overwrite(
                    "root.txt",
                    "Hello world (from upstream)\n",
                )
            ]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();
            upstream.push().unwrap();
        },
        "sub",
    )
        .unwrap()
}

#[test]
pub fn reproduce_merge_revert_error() {
    let test = base("reproduce_merge_revert_error");

    test.get_upstream().commit_changes(vec!(FileAction::overwrite(
        "root2.txt",
        "More root data\n",
    )), "A commit to use as a base for merging");

    test.get_upstream().commit_changes(vec!(FileAction::overwrite(
        "root.txt",
        "Changed upstream\n",
    )), "Another upstream commit");

//    test.get_upstream().commit_changes(vec!(FileAction::overwrite(
//        "sub/new-upstream.txt",
//        "A new file from upstream\n",
//    )), "Add subgit file (upstream)");

    test.get_upstream().push().unwrap();
    test.get_upstream().checkout("HEAD~").unwrap();
    test.get_upstream().checkout_adv(vec!["-b", "temp1"]).unwrap();


    test.get_upstream().commit_changes(vec!(FileAction::overwrite(
        "sub/new-upstream.txt",
        "A new file from upstream\n",
    )), "Add subgit file (upstream)");

//    test.get_upstream().commit_changes(vec!(FileAction::overwrite(
//        "root.txt",
//        "Changed upstream\n",
//    )), "Another upstream commit");


    test.get_upstream().checkout("master").unwrap();
    test.get_upstream().merge(&["temp1"]).unwrap();
    test.get_upstream().push().unwrap();


    std::thread::sleep(Duration::new(5, 0));

    test.get_subgit().commit_changes(vec!(FileAction::overwrite(
        "hello.txt",
        "A change\n",
    )), "Changed subgit file (downstream)");

    test.get_subgit().pull().unwrap();
    test.get_subgit().push().unwrap();

    test.get_upstream().pull().unwrap();

    test.do_then_verify(|_,_| Ok(()));
}

