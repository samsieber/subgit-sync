extern crate log;
extern crate simplelog;
extern crate subgit_sync;

mod harness;
mod util;

use crate::harness::*;
use std::time::Duration;

#[test]
pub fn force_push_from_upstream() {
    let test = TestWrapper::new(
        "force_push_from_upstream",
        |upstream| {
            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();

            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (again, from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("Second Commit from Upstream").unwrap();

            upstream.push().unwrap();
        },
        "sub",
    )
    .unwrap();

    test.do_then_verify(|upstream, downstream| {
        upstream
            .command_output(vec!["reset", "--hard", "HEAD~"])
            .unwrap();

        upstream.update_working(vec![FileAction::overwrite(
            "sub/hello.txt",
            "Hello world (newly, from upstream)",
        )]);
        upstream.add(".").unwrap();
        upstream.commit("New Second Commit from Upstream").unwrap();
        upstream
            .push_adv(vec!["origin", "+HEAD:refs/heads/master"])
            .unwrap();

        std::thread::sleep(Duration::new(2, 0));

        downstream.command_output(vec!["fetch", "--all"]).unwrap();
        downstream
            .command_output(vec!["reset", "--hard", "origin/master"])
            .unwrap();

        Ok(())
    });
}

#[test]
pub fn force_push_from_downstream_should_fail() {
    let test = TestWrapper::new(
        "force_push_from_downstream_should_fail",
        |upstream| {
            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();

            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (again, from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("Second Commit from Upstream").unwrap();

            upstream.push().unwrap();
        },
        "sub",
    )
    .unwrap();

    let downstream = test.get_subgit();

    downstream
        .command_output(vec!["reset", "--hard", "HEAD~"])
        .unwrap();

    downstream.update_working(vec![FileAction::overwrite(
        "sub/hello.txt",
        "Hello world (newly, from upstream)",
    )]);
    downstream.add(".").unwrap();
    downstream.commit("New Commit from Downstream").unwrap();
    if let Ok(_) = downstream.push_adv(vec!["origin", "+HEAD:refs/heads/master"]) {
        assert!(false);
    }
}

#[test]
pub fn delete_branch_from_downstream() {
    let test = TestWrapper::new(
        "delete_branch_from_downstream",
        |upstream| {
            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();

            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (again, from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("Second Commit from Upstream").unwrap();

            upstream.push().unwrap();
        },
        "sub",
    )
    .unwrap();

    test.do_then_verify(|upstream, _downstream| {
        upstream.checkout("HEAD~").unwrap();

        upstream.update_working(vec![FileAction::overwrite(
            "sub/hello.txt",
            "Hello world (newly, from upstream)",
        )]);
        upstream.add(".").unwrap();
        upstream.commit("New Second Commit from Upstream").unwrap();
        upstream
            .push_adv(vec!["origin", "+HEAD:refs/heads/to_delete"])
            .unwrap();
        upstream.checkout("master").unwrap();

        std::thread::sleep(Duration::new(2, 0));

        Ok(())
    });

    test.do_then_verify(|_upstream, downstream| {
        downstream.push_adv(vec!["origin", ":to_delete"])?;
        Ok(())
    });

    test.get_subgit()
        .command_output(vec!["fetch", "--all"])
        .unwrap();
    assert!(test
        .get_subgit()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "to_delete"))
            .unwrap()
            .trim(),
        ""
    );

    test.get_upstream()
        .command_output(vec!["fetch", "--all"])
        .unwrap();
    test.get_upstream()
        .command_output(vec!["remote", "prune", "origin"])
        .unwrap();
    assert!(test
        .get_upstream()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_upstream()
            .command_output(vec!("show-ref", "to_delete"))
            .unwrap()
            .trim(),
        ""
    );
}

#[test]
pub fn delete_branch_from_upstream() {
    let test = TestWrapper::new(
        "delete_branch_from_upstream",
        |upstream| {
            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();

            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (again, from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("Second Commit from Upstream").unwrap();

            upstream.push().unwrap();
        },
        "sub",
    )
    .unwrap();

    test.do_then_verify(|upstream, _downstream| {
        upstream.checkout("HEAD~").unwrap();

        upstream.update_working(vec![FileAction::overwrite(
            "sub/hello.txt",
            "Hello world (newly, from upstream)",
        )]);
        upstream.add(".").unwrap();
        upstream.commit("New Second Commit from Upstream").unwrap();
        upstream
            .push_adv(vec!["origin", "+HEAD:refs/heads/to_delete"])
            .unwrap();
        upstream.checkout("master").unwrap();

        std::thread::sleep(Duration::new(2, 0));

        Ok(())
    });

    test.do_then_verify(|upstream, _downstream| {
        upstream.push_adv(vec!["origin", ":to_delete"])?;
        std::thread::sleep(Duration::new(2, 0));
        Ok(())
    });

    test.get_subgit()
        .command_output(vec!["fetch", "--all"])
        .unwrap();
    test.get_subgit()
        .command_output(vec!["remote", "prune", "origin"])
        .unwrap();
    assert!(test
        .get_subgit()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "to_delete"))
            .unwrap()
            .trim(),
        ""
    );

    test.get_upstream()
        .command_output(vec!["fetch", "--all"])
        .unwrap();
    assert!(test
        .get_upstream()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_upstream()
            .command_output(vec!("show-ref", "to_delete"))
            .unwrap()
            .trim(),
        ""
    );
}
