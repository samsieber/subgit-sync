extern crate subgit_rs;
extern crate log;
extern crate simplelog;

mod harness;
mod util;

use harness::*;
use std::time::Duration;

fn base(name: &str) -> TestWrapper {
    TestWrapper::new(name, |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Hello world (from upstream)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap()
}

#[test]
pub fn push_single_on_master() {
    let test = base("push_single_on_master");

    test.verify_push_changes_and_pull_in_other(GitType::Subgit, vec![
        FileAction::overwrite("test.txt", "hello from subgit")
    ], "First subgit commit")
}

#[test]
pub fn push_multiple_on_master() {
    let test = base("push_multiple_on_master");

    test.do_then_verify(|upstream, downstream| {
        downstream.update_working(vec![FileAction::overwrite("test.txt", "hello from subgit")]);
        downstream.add(".").unwrap();
        downstream.commit("First commit").unwrap();

        downstream.update_working(vec![FileAction::overwrite("test2.txt", "hello from subgit 2")]);
        downstream.add(".").unwrap();
        downstream.commit("Second commit").unwrap();

        downstream.push().unwrap();

        std::thread::sleep(Duration::new(2,0));

        upstream.pull().unwrap();

        Ok(())
    })
}

#[test]
pub fn push_orphaned_commit() {
    let test = base("push_orphaned_commit");

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        downstream.update_working(vec![FileAction::overwrite("testing.txt", "Applicable")]);
        downstream.add(".").unwrap();
        downstream.commit("First Commit from Upstream").unwrap();
        downstream.push_adv(["-u", "origin", "orphaned"]).unwrap();

        std::thread::sleep(Duration::new(2,0));

        upstream.pull().unwrap();
        upstream.checkout("orphaned").unwrap();

        Ok(())
    })
}