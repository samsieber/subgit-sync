extern crate log;
extern crate simplelog;
extern crate subgit_sync;

use test_harness::harness::*;
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
        GitType::Upstream,
        vec![FileAction::overwrite("sub/test.txt", "hello from upstream")],
        "First pushed upstream commit",
    )
}

#[test]
pub fn push_multiple_on_master() {
    let test = base("push_multiple_on_master");

    test.do_then_verify(|upstream, downstream| {
        upstream.update_working(vec![FileAction::overwrite(
            "sub/test.txt",
            "hello from upstream",
        )]);
        upstream.add(".").unwrap();
        upstream
            .commit("First commit to push from upstream")
            .unwrap();

        upstream.update_working(vec![FileAction::overwrite(
            "sub/test2.txt",
            "hello from upstream 2",
        )]);
        upstream.add(".").unwrap();
        upstream
            .commit("Second commit to push from upstream")
            .unwrap();

        upstream.push().unwrap();

        std::thread::sleep(Duration::new(3, 0));

        downstream.pull().unwrap();

        Ok(())
    })
}

#[test]
pub fn push_orphaned_commit() {
    let test = base("push_orphaned_commit");

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![FileAction::overwrite("sub/testing.txt", "Applicable")]);
        upstream.add(".").unwrap();
        upstream
            .commit("First Commit from orphaned upstream")
            .unwrap();
        upstream.push_adv(["-u", "origin", "orphaned"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.pull().unwrap();
        downstream.checkout("orphaned").unwrap();

        Ok(())
    })
}

#[test]
pub fn push_new_tip_with_existing_sha() {
    let test = base("push_tip_with_existing_sha");

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout_adv(["-b", "second"]).unwrap();
        upstream.push_adv(["-u", "origin", "second"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.pull().unwrap();
        downstream.checkout("second").unwrap();

        assert_eq!(
            upstream
                .command_output(vec!["rev-parse", "second"])
                .unwrap(),
            upstream
                .command_output(vec!["rev-parse", "master"])
                .unwrap()
        );

        assert_eq!(
            downstream
                .command_output(vec!["rev-parse", "second"])
                .unwrap(),
            downstream
                .command_output(vec!["rev-parse", "master"])
                .unwrap()
        );

        Ok(())
    });
}

#[test]
pub fn push_new_tips_with_existing_sha_u() {
    let test = base("push_new_tips_with_existing_sha_u");

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout_adv(["-b", "second"]).unwrap();
        upstream.checkout_adv(["-b", "third"]).unwrap();

        upstream
            .push_adv(["origin", "second:second", "third:third"])
            .unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.pull().unwrap();
        downstream.checkout("third").unwrap();

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
        upstream.checkout("master").unwrap();
        upstream.checkout_adv(["-b", "second"]).unwrap();
        upstream.update_working(vec![FileAction::overwrite("sub/second.txt", "sec")]);
        upstream.add(".").unwrap();
        upstream.commit("Commit from second branch").unwrap();

        upstream.checkout("master").unwrap();
        upstream.checkout_adv(["-b", "third"]).unwrap();
        upstream.update_working(vec![FileAction::overwrite("sub/third.txt", "thi")]);
        upstream.add(".").unwrap();
        upstream.commit("Commit from third branch").unwrap();

        upstream
            .push_adv(["origin", "second:second", "third:third"])
            .unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.pull().unwrap();
        downstream.checkout("third").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout("second").unwrap();
        downstream.checkout("second").unwrap();

        Ok(())
    });
}

#[test]
pub fn push_existing_on_master() {
    let test = base("push_existing_on_master");

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout_adv(["-b", "second"]).unwrap();

        upstream.update_working(vec![FileAction::overwrite("sub/second.txt", "sec")]);
        upstream.add(".").unwrap();
        upstream.commit("Commit from second branch").unwrap();

        upstream.push_adv(["origin", "second:second"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.pull().unwrap();
        downstream.checkout("second").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        upstream.push_adv(["origin", "second:master"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.checkout("master").unwrap();
        downstream.pull().unwrap();

        Ok(())
    });
}

#[test]
pub fn push_and_then_delete_branch() {
    let test = base("push_and_then_delete_branch");

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout_adv(["-b", "second"]).unwrap();
        upstream.update_working(vec![FileAction::overwrite("second.txt", "sec")]);
        upstream.add(".").unwrap();
        upstream.commit("Commit from second branch").unwrap();
        upstream.push_adv(["origin", "second:second"]).unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.pull().unwrap();
        downstream.checkout("second").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        upstream.push_adv(["origin", ":second"]).unwrap();
        assert_eq!(
            "",
            upstream
                .command_output(vec!["ls-remote", "--heads", "origin", "second"])
                .unwrap()
        );

        std::thread::sleep(Duration::new(2, 0));

        downstream.command_output(vec!["fetch", "--all"]).unwrap();
        assert_eq!(
            "",
            downstream
                .command_output(vec!["ls-remote", "--heads", "origin", "second"])
                .unwrap()
        );

        Ok(())
    });
}
