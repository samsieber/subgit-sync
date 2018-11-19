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
            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();
            upstream.push().unwrap();
        },
        "sub",
    )
    .unwrap()
}

#[test]
pub fn push_single_on_master() {
    let test = base("push_single_on_master");

    test.verify_push_changes_and_pull_in_other(
        GitType::Subgit,
        vec![FileAction::overwrite("test.txt", "hello from subgit")],
        "First subgit commit",
    )
}

#[test]
pub fn push_multiple_on_master() {
    let test = base("push_multiple_on_master");

    test.do_then_verify(|upstream, downstream| {
        downstream.update_working(vec![FileAction::overwrite("test.txt", "hello from subgit")]);
        downstream.add(".").unwrap();
        downstream.commit("First commit").unwrap();

        downstream.update_working(vec![FileAction::overwrite(
            "test2.txt",
            "hello from subgit 2",
        )]);
        downstream.add(".").unwrap();
        downstream.commit("Second commit").unwrap();

        downstream.push().unwrap();

        std::thread::sleep(Duration::new(2, 0));

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
        downstream
            .commit("First Commit from orphaned downstream")
            .unwrap();
        downstream.push_adv(["-u", "origin", "orphaned"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        upstream.pull().unwrap();
        upstream.checkout("orphaned").unwrap();

        Ok(())
    })
}

#[test]
pub fn push_new_tip_with_existing_sha() {
    let test = base("push_new_tip_with_existing_sha");

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout_adv(["-b", "second"]).unwrap();
        downstream.push_adv(["-u", "origin", "second"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        upstream.pull().unwrap();
        upstream.checkout("second").unwrap();

        assert_eq!(
            downstream
                .command_output(vec!["rev-parse", "second"])
                .unwrap(),
            downstream
                .command_output(vec!["rev-parse", "master"])
                .unwrap()
        );

        assert_eq!(
            upstream
                .command_output(vec!["rev-parse", "second"])
                .unwrap(),
            upstream
                .command_output(vec!["rev-parse", "master"])
                .unwrap()
        );

        Ok(())
    });
}

#[test]
pub fn push_new_tips_with_existing_sha() {
    let test = base("push_new_tips_with_existing_sha");

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout_adv(["-b", "second"]).unwrap();
        downstream.checkout_adv(["-b", "third"]).unwrap();

        downstream
            .push_adv(["origin", "second:second", "third:third"])
            .unwrap();

        std::thread::sleep(Duration::new(2, 0));

        upstream.pull().unwrap();
        upstream.checkout("third").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout("second").unwrap();
        upstream.checkout("second").unwrap();

        Ok(())
    });
}

#[test]
pub fn push_new_tips_with_new_shas() {
    let test = base("push_new_tips_with_new_shas");

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout("master").unwrap();
        downstream.checkout_adv(["-b", "second"]).unwrap();
        downstream.update_working(vec![FileAction::overwrite("second.txt", "sec")]);
        downstream.add(".").unwrap();
        downstream.commit("Commit from second branch").unwrap();

        downstream.checkout("master").unwrap();
        downstream.checkout_adv(["-b", "third"]).unwrap();
        downstream.update_working(vec![FileAction::overwrite("third.txt", "thi")]);
        downstream.add(".").unwrap();
        downstream.commit("Commit from third branch").unwrap();

        downstream
            .push_adv(["origin", "second:second", "third:third"])
            .unwrap();

        std::thread::sleep(Duration::new(2, 0));

        upstream.pull().unwrap();
        upstream.checkout("third").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout("second").unwrap();
        upstream.checkout("second").unwrap();

        Ok(())
    });
}

#[test]
pub fn push_existing_on_master() {
    let test = base("push_existing_on_master");

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout_adv(["-b", "second"]).unwrap();

        downstream.update_working(vec![FileAction::overwrite("second.txt", "sec")]);
        downstream.add(".").unwrap();
        downstream.commit("Commit from second branch").unwrap();

        downstream.push_adv(["origin", "second:second"]).unwrap();

        upstream.pull().unwrap();
        upstream.checkout("second").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        downstream.push_adv(["origin", "second:master"]).unwrap();

        upstream.checkout("master").unwrap();
        upstream.pull().unwrap();

        Ok(())
    });
}

#[test]
pub fn push_and_then_delete_branch() {
    let test = base("push_and_then_delete_branch");

    test.do_then_verify(|upstream, downstream| {
        downstream.checkout_adv(["-b", "second"]).unwrap();
        downstream.update_working(vec![FileAction::overwrite("second.txt", "sec")]);
        downstream.add(".").unwrap();
        downstream.commit("Commit from second branch").unwrap();
        downstream.push_adv(["origin", "second:second"]).unwrap();

        upstream.pull().unwrap();
        upstream.checkout("second").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        downstream.push_adv(["origin", ":second"]).unwrap();
        assert_eq!(
            "",
            downstream
                .command_output(vec!["ls-remote", "--heads", "origin", "second"])
                .unwrap()
        );

        upstream.command_output(vec!["fetch", "--all"]).unwrap();
        assert_eq!(
            "",
            upstream
                .command_output(vec!["ls-remote", "--heads", "origin", "second"])
                .unwrap()
        );

        Ok(())
    });
}
