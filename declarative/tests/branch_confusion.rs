use declarative::*;

#[test]
pub fn feature_branch_master_ff() {
    let config : TestConfig = config_for!("feature_branch_master_ff");
    let (target, executor) = target_executor();

    // Initial setup
    let mut tree = CommitTree::new();
    let root = tree.root("root", target.empty());
    let start1 = tree.commit("start1", target.upstream(), &root);
    let start2 = tree.commit("start2", target.subgit(), &start1);
    let a = tree.commit("_A_", target.both(), &start2);
    tree.branch("master", &a);

    let test = config.run_setup(tree.clone(), executor);

    // This is what the tree should look like
    let mut expected = tree;
    let b = expected.commit("_B_", target.both(), &a);
    let feat = expected.commit("_FEAT_", target.upstream(), &b);
    expected.branch("feature", &feat);
    let c = expected.commit("_C_", target.subgit(), &b);
    expected.branch("master", &c);
    let expected = expected;

    // Upstream: Commit to master
    let up_master = test.upstream_consumer("up_master");
    let b_prime = up_master
        .commit(b).0;
    up_master.push("master");

    // Upstream: Branch from master and push a new feature branch
    let up_feature = test.upstream_consumer("up_feature");
    let feat_prime = up_feature
        .checkout_branch("feature")
        .commit(feat).0;
    up_feature.push("feature");

    sleep(6);

    // Subgit : Commit to master
    let sg_master = test.subgit_consumer("sg_master");
    let c_prime = sg_master.commit(c).0;
    sg_master.push("master");

    test.verify(expected);
}
#[test]
pub fn feature_branch_master_merge() {
    let config : TestConfig = config_for!("feature_branch_master_merge");
    let (target, executor) = target_executor();

    // Initial setup
    let mut tree = CommitTree::new();
    let root = tree.root("root", target.empty());
    let start1 = tree.commit("start1", target.upstream(), &root);
    let start2 = tree.commit("start2", target.subgit(), &start1);
    let a = tree.commit("_A_", target.both(), &start2);
    tree.branch("master", &a);

    let test = config.run_setup(tree.clone(), executor);

    // This is what the tree should look like
    let mut expected = tree;
    let b = expected.commit("_B_", target.both(), &a);
    let c = expected.commit("_C_", target.upstream(), &a);
    expected.branch("feature", &c);
    let d = expected.commit("_D_", target.subgit(), &a);
    let e = expected.merge_2("_E_", &d, &b);
    expected.branch("master", &e);
    let expected = expected;

    // Upstream: Commit to master
    let dev_master = test.upstream_consumer("dev_master");
    let b_prime = dev_master
        .commit(b).0;

    // Upstream: Branch from master and push a new feature branch
    let dev_feature = test.upstream_consumer("dev_feature");
    let c_prime = dev_feature
        .checkout_branch("feature")
        .commit(c).0;

    // Subgit : Commit to mastermaster
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