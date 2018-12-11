use std::path::PathBuf;
use subgit_sync::SetupRequest;
use crate::executor::Runnable;
use crate::tree::CommitTree;
use crate::git::GitWrapper;
use crate::tree::TreeRecord;
use crate::git::InternalGit;
use crate::git::Consumer;

#[derive(Debug)]
pub struct TestConfig {
    pub test: String,
    pub module: String,
}

pub trait Executor {
    fn get_root(&self, harness: &TestConfig) -> PathBuf;

    /// Setup the args
    /// At this point, subgit has been setup, and the initial commits to the upstream have been committed (but not pushed)
    fn setup_args(&self, harness: &TestConfig) -> SetupRequest;
}

pub struct Test {
    /// The root of the test, or where to clone consumers into
    path: PathBuf,
    /// Path for cloning the upstream
    upstream_url: String,
    /// Path for cloning the subgit
    subgit_url: String,
    /// Path to the folder in the upstream that will be republished
    repub_base: String,
}

impl TestConfig {
    pub fn new(test: impl AsRef<str>, module_path: impl AsRef<str>) -> TestConfig {
        TestConfig {
            test: test.as_ref().to_owned(),
            module: module_path.as_ref().to_owned(),
        }
    }

    pub fn run_setup(self, commit_tree: CommitTree, executor: impl Executor) -> Test {
        let root = executor.get_root(&self);
        crate::util::clear_path(&root);

        let upstream_bare = crate::util::init_bare_repo("upstream.git", &root).expect("Could not init upstream bare repo");
        let upstream_working = crate::util::clone(&root, upstream_bare).unwrap();
        let git = GitWrapper::new(upstream_working.clone());
        commit_tree.commit_tree(&git, &mut TreeRecord::new());
        git.push_all();
        git.set_head("master".to_owned());
        crate::util::clear_path(upstream_working);
        crate::util::init_bare_repo("subgit.git", &root).expect("Could not init subgit bare repo");
        let setup = executor.setup_args(&self);
        setup.clone().run(&root);

        Test {
            path: root,
            upstream_url: setup.upstream_working_clone_url.unwrap_or(format!("file://{}", setup.upstream_git_location)),
            subgit_url: setup.subgit_working_clone_url.unwrap_or(format!("file://{}", setup.subgit_git_location)),
            repub_base: format!("{}/", setup.upstream_map_path),
        }
    }
}

impl Test {
    pub fn verify(&self, tree: CommitTree) {
        let branch_map = tree.reify();
        let compare_root = self.path.join("C_OMPARE");
        let upstream = Compare{
            url: &self.upstream_url,
            base: "",
            name: "upstream",
            path: "compare-upstream",
            comp: self.path.join(&compare_root),
        };
        let subgit = Compare {
            url: &self.subgit_url,
            base: &self.repub_base,
            name: "subgit",
            path: "compare-subgit",
            comp: self.path.join(&compare_root).join(&self.repub_base),
        };
        let comparisons = vec!(upstream, subgit);
        comparisons.iter().for_each(|v| {
            crate::util::clone_url(&self.path, v.url, v.path).unwrap();
            crate::util::command(self.path.join(v.path), "git", &["checkout", "-b", "get-off-the-master-branch"]).unwrap();
        });
        let diff_messages : Vec<Diff> = branch_map.into_iter().map(|(branch, files)| {
            crate::util::clear_path(&compare_root);
            crate::util::write_files(&compare_root, files).expect(&format!("Couldn't write files for {} branch", &branch));
            comparisons.clone().into_iter().flat_map({
                let branch = branch.clone();
                move |c| {
                    crate::util::command(self.path.join(c.path), "git", &["checkout", &branch]).unwrap();
                    let result = crate::util::compare_dir_content(&c.comp, self.path.join(&c.path));
                    if result.len() > 0 {
                        Some(Diff {
                            branch: branch.clone(),
                            repo: c.name.to_string(),
                            message: result,
                        })
                    } else {
                        None
                    }
                }
            })
        }).flatten().collect();

        if diff_messages.len() > 0 {
            eprintln!("Found differences!: ");
            diff_messages.into_iter().for_each(|diff| {
                eprintln!("-------------------\nRepo: {}, Branch: {}, Message:\n{}\n\n", diff.repo, diff.branch, diff.message);
            });
            panic!("Differences were found! See above");
        }
    }

    pub fn upstream_consumer<N: AsRef<str>>(&self, name: N) -> Consumer {
        self.new_consumer(name.as_ref().to_owned(), self.upstream_url.clone(), None)
    }

    pub fn subgit_consumer<N: AsRef<str>>(&self, name: N) -> Consumer {
        self.new_consumer(name.as_ref().to_owned(), self.subgit_url.clone(), Some(self.repub_base.clone()))
    }

    fn new_consumer(&self, name: String, clone_url: String, filter: Option<String>) -> Consumer {
        crate::util::clone_url(&self.path, clone_url, &name).unwrap();
        Consumer::new(self.path.join(&name), filter)
    }
}

struct Diff {
    branch: String,
    repo: String,
    message: String,
}

#[derive(Clone)]
struct Compare<'a,'b > {
    url: &'a str,
    base: &'b str,
    name: &'static str,
    path: &'static str,
    comp: PathBuf,

}