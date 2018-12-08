extern crate log;
extern crate simplelog;
extern crate subgit_sync;

use test_harness::harness::*;
use std::time::Duration;

#[test]
pub fn import_good_ref_but_not_bad_refs() {
    let test = TestWrapper::new(
        "import_good_ref_but_not_bad_ref",
        |upstream| {
            upstream.update_working(vec![FileAction::overwrite(
                "sub/hello.txt",
                "Hello world (from upstream)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("First Commit from Upstream").unwrap();
            upstream.push().unwrap();

            let head = upstream
                .command_output(vec!["rev-parse", "HEAD"])
                .unwrap()
                .trim()
                .to_owned();
            upstream.checkout(head).unwrap();

            upstream.update_working(vec![FileAction::overwrite(
                "sub/again.txt",
                "Hello world (from upstream again)",
            )]);
            upstream.add(".").unwrap();
            upstream.commit("Second Commit from Upstream").unwrap();

            upstream
                .push_adv(vec!["origin", &format!("HEAD:{}", "refs/test/bad")])
                .unwrap();

            upstream
                .command_output(vec!["tag", "-a", "tag_heavy", "-m", "my heavy tag"])
                .unwrap();
            upstream.command_output(vec!["tag", "tag_light"]).unwrap();

            upstream.checkout("master").unwrap();
        },
        "sub",
    )
    .unwrap();

    assert!(test
        .get_subgit()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "bad"))
            .unwrap()
            .trim(),
        ""
    );
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "tag_heavy"))
            .unwrap()
            .trim(),
        ""
    );
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "tag_light"))
            .unwrap()
            .trim(),
        ""
    );
}

#[test]
pub fn upstream_push_good_ref_but_not_bad_refs() {
    let test = TestWrapper::new(
        "upstream_push_good_ref_but_not_bad_refs",
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
    .unwrap();

    test.do_then_verify(|upstream, _downstream| {
        let head = upstream
            .command_output(vec!["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_owned();
        upstream.checkout(head).unwrap();

        upstream.update_working(vec![FileAction::overwrite(
            "sub/again.txt",
            "Hello world (from upstream again)",
        )]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream
            .push_adv(vec!["origin", &format!("HEAD:{}", "refs/test/bad")])
            .unwrap();

        upstream
            .command_output(vec!["tag", "-a", "tag_heavy", "-m", "my heavy tag"])
            .unwrap();
        upstream.command_output(vec!["tag", "tag_light"]).unwrap();

        upstream.checkout("master").unwrap();

        std::thread::sleep(Duration::new(2, 0));

        Ok(())
    });

    assert!(test
        .get_subgit()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "bad"))
            .unwrap()
            .trim(),
        ""
    );
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "tag_heavy"))
            .unwrap()
            .trim(),
        ""
    );
    assert_eq!(
        test.get_subgit()
            .command_output(vec!("show-ref", "tag_light"))
            .unwrap()
            .trim(),
        ""
    );
}

#[test]
pub fn downstream_push_good_ref_but_not_bad_refs() {
    let test = TestWrapper::new(
        "downstream_push_good_ref_but_not_bad_refs",
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
    .unwrap();

    test.do_then_verify(|_upstream, downstream| {
        let head = downstream
            .command_output(vec!["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_owned();
        downstream.checkout(head).unwrap();

        downstream.update_working(vec![FileAction::overwrite(
            "again.txt",
            "Hello world (from downstream)",
        )]);
        downstream.add(".").unwrap();
        downstream.commit("Second Commit from Upstream").unwrap();

        downstream
            .push_adv(vec!["origin", &format!("HEAD:{}", "refs/test/bad")])
            .unwrap();

        downstream
            .command_output(vec!["tag", "-a", "tag_heavy", "-m", "my heavy tag"])
            .unwrap();
        downstream.command_output(vec!["tag", "tag_light"]).unwrap();

        downstream.checkout("master").unwrap();

        Ok(())
    });

    assert!(test
        .get_subgit()
        .command_output(vec!("show-ref", "master"))
        .unwrap()
        .trim()
        .contains("master"));
    assert_eq!(
        test.get_upstream()
            .command_output(vec!("show-ref", "bad"))
            .unwrap()
            .trim(),
        ""
    );
    assert_eq!(
        test.get_upstream()
            .command_output(vec!("show-ref", "tag_heavy"))
            .unwrap()
            .trim(),
        ""
    );
    assert_eq!(
        test.get_upstream()
            .command_output(vec!("show-ref", "tag_light"))
            .unwrap()
            .trim(),
        ""
    );
}
