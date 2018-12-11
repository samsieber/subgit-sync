use declarative::*;

#[test]
pub fn feature_branch_master() {
    let config : TestConfig = config_for!("feature_branch_master");
    let (target, executor) = target_executor();

    let mut tree = CommitTree::new();
    let root = tree.root("root", target.empty());
    let start1 = tree.commit("start1", target.upstream(), &root);
    let start2 = tree.commit("start2", target.subgit(), &start1);
    let a = tree.commit("_A_", target.both(), &start2);
    tree.branch("master", &a);

    let test = config.run_setup(tree.clone(), executor);

    let mut expected = tree;
    let b = expected.commit("_B_", target.both(), &a);
    let c = expected.commit("_C_", target.upstream(), &a);
    expected.branch("feature", &c);
    let d = expected.commit("_D_", target.subgit(), &a);
    let e = expected.merge_2("_E_", &d, &b);
    expected.branch("master", &e);
    let expected = expected;



    let dev_master = test.upstream_consumer("dev_master");
    let b_prime = dev_master
        .commit(b).0;

    let dev_feature = test.upstream_consumer("dev_feature");
    let c_prime = dev_feature
        .checkout_branch("feature")
        .commit(c).0;

    let sg_merger = test.subgit_consumer("sg_merger");
    let d_prime = sg_merger.commit(d).0;

    // Push B -> A onto master
    dev_master.push("master");
    // Push C -> A onto master
    dev_feature.push("feature");
    // Push D -> A onto master in the form of:
    //  E -> D -> A
    //   \-> B ->/
    sleep(3);
    sg_merger.pull_merge().push("master");
    sleep(3);

    test.verify(expected);
}
