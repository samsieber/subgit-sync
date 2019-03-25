# SubGit Sync [BETA - Still might eat commits]

Subgit is a server side hook that lets you synchronize two repositories, making a subdirectory in one the top level directory in the other. It seamlessly and continuosly copies commits between the two, to the point that the end git users can treat both repositories as completely separate.

Consider using this when:
1) You have a subset of some repository you'd like to expose to more people, but without exposing the top level code
2) You self host the git repos of interest
3) You want this to be completely transparent to the end users of the top level repo
4) You only need on branch synced (it can't handle more than that *right now*)

### Basic Anatomy of Subgit

Under the hood, it is a single binary with three distinct functions:
  * It can run the setup and initial synchronization, copying itself to into the subgit and upstream repositories in the appropriate locations
  * Can act as the update hook in the subgit repository
  * Can act as the post-receive hook in the upstream hook
  
And example invocation of the program would be `subgit-rs some/upstream/repo.git target/subgit.git dir/to/republish`,
which would setup the `target/subgit.git` to be a synchronized copy of the `dir/to/republish` folder from the `some/upstream/repo.git` repository.

This project is implemented using the Rust programming language, and it compiles down to a single binary. It uses the 
libgit2 rust bindings and a few other rust dependencies (logging, command line option parsing, etc).WIP

### Limitations

Subgit **will occassionally & silenly merge in branches in the upstream** when tracking multiple branches (see https://github.com/samsieber/subgit-sync/issues/2) if you're pushing to the subgit - luckily there is a cli option to control what branches / refspecs will be sync'd. **Only use this on one branch**

Subgit isn't very well tested. Though it's currently being used, it hasn't been very used, and latent commit eating bugs may exist.

Subgit only works on linux hosts. It *might* work on Mac. Probably not on windows.

## Example setup for Gitlab

If you have a repository you'd like to set this up on in a self hosted GitLab, here are the steps you'd take to publish the `something/to/share` folder from the `internal/private` repo as the `public/shared` repo. This assumes that repository data is stored at `/var/opt/gitlab/git-data/repositories`, and that the server is accessible at `gitlab.example.com`.

### Prep Work:

1) As a GitLab administrator, create a gitlab user - this is the user that will be used when pushing hooks. For this example, let's call it `syncer`.
2) On the GitLab server, create an ssh key for the git user (the git user is the user used to execute the server side hooks) - add this to the user you created in step one.
3) Create a new repo - `public/shared` (*Don't commit anything to it*)
4) Add the `syncer` user with `Maintainer` access to **both** the `public/shared` and the `internal/private`
5) Create the `custom_hooks` folder for the `internal/private` repo (`/var/opt/gitlab/git-data/repositories/internal/private/custom_hooks`) if it doesn't already exist
6) Create the `custom_hooks` folder for the `public/shared` repo (`/var/opt/gitlab/git-data/repositories`)
7) You're now ready to run the setup

### The Setup Command
On the GitLab server, **As the `git` user**, run the following ugly command: ```./subgit-sync                                                                   /var/opt/gitlab/git-data/repositories/internal/private.git /var/opt/gitlab/git-data/repositories/public/shared.git        somthing/to/share -U git@gitlab.simplifile.com:internal/private.git -H custom_hooks/post-receive -u git@gitlab.example.com:public/shared.git -h custom_hooks/update -r GL_USERNAME:syncer -m refs/heads/master,HEAD```

The command is the hook itself (it copies itself). Lets break down the arguments
 1) Where the upstream bare lives on disk
 2) Where the subgit/downstream/subfolder bare repo lives on disk
 3) Which folder from the upstream to publish as the root of the subgit
 4) The -U tells the hook which URL to use when pushing commits to the upstream.
 5) The -u tells the hook which URL to use when pushing commits to the subgit.
 6) The -H tells the hook where to place itself (relative path) in the upstream repo.
 7) The -h tells the hook where to place itself (relative path) in the subgit repo.
 8) The -r tells the hook that it's recursing if the `GL_USERNAME` environment variable is set to `syncer`
 9) The -m tells the hook which refspecs to manage (sync)
 
## Usage Syntax / Help (Copied Verbatim)
```
subgit-sync 0.3.2
Sam Sieber <swsieber@gmail.com>
Installs git hooks to republish a path of repository (henceforth: upstream) as it's own top-level repository
(henceforth: subgit) and synchronize commits between them, using the upstream as the source of truth

It's designed to be used on repositories that reside on the same filesystem, for which you have admin access.

It places an server-side update hook in the subgit repo, and a server-side post-receive hook in the upstream repo. The
update hooks synchronously exports commits from the subgit repo to the upstream repo, refusing the push if the upstream
cannot be updated. The upstream hook asynchronously requests the subgit to import the newly pushed commits

USAGE:
    subgit-sync [FLAGS] [OPTIONS] <upstream_git_location> <subgit_git_location> <upstream_map_path>

FLAGS:
    -w, --use_whitelist_recursion_detection    Disables recursive hook call checking This cannot be used with a custom
                                               subgit_working_clone_url due to the infinite recursion that occurs when
                                               both the upstream hook and subgit hook are triggered during
                                               synchronization
        --help                                 Prints help information
    -V, --version                              Prints version information

OPTIONS:
    -p, --subgit_map_path <subgit_map_path>
            The path in the subgit repo to place the republished files from upstream Defaults to the root of the
            repository
    -l, --log_level <log_level>
            The log level to use when logging to file from the hooks

    -f, --log_file <log_file>
            The path of the log file to write to during setup

    -H, --upstream_hook_path <upstream_hook_path>
            The hook path to use in the upstream repository [default: hooks/post-receive]

    -h, --subgit_hook_path <subgit_hook_path>
            The hook path to use in the subgit repository [default: hooks/update]

    -U, --upstream_working_clone_url <upstream_working_clone_url>
            Specify an external url to push changes to, when exporting commits to the upstream from the subgit If not
            set, uses the file path to the upstream repo
    -u, --subgit_working_clone_url <subgit_working_clone_url>
            Specify an external url to push changes to, when import commits in the subgit from the upstream If not set,
            uses a modified subgit bare repo that bypasses the server hooks
    -r, --env_based_recursion_detection <env_based_recursion_detection>
            Specifies an environment variable name and value to look for when trying to detect recursive hook calls
            Defaults to using the --push-option added in git 2.10 The value must be in the form of ENV_NAME:ENV_VALUE
            For example, for gitlab servers, you'd most likely use 'GL_USERNAME:git' as the value
    -m, --match_ref <match_ref>
            Only operate on the refs that start with these values - pass in a comma separated list [default:
            refs/heads/,HEAD]

ARGS:
    <upstream_git_location>    The location of the bare upstream repository on disk
    <subgit_git_location>      The location of the bare subgit repository on disk
    <upstream_map_path>        The path in the upstream repository to republish as the root in the subgit repository
```

## Synchronization Logic
### General Flow

After the initial setup, there are two flows - 
you push to the upstream and it propagates to the subgit, 
or you push to the subgit and it propagates to the upstream

#### Pushing to the subgit

When pushing to the subgit, the pre-receive hook checks that it's up to date with the upstream repo.
If it's not, it fails the push, *after importing the latest commits from upstream, so the that pusher may pull and try again*.

If it is up-to-date, then it tries to update the upstream repo with the new commits.
If it succeeds, then the push is accepted.

#### Pushing to the master

Since the subgit aborts any changes that don't get imported into the upstream, the commits from the upstream should 
always be safe to import into the subgit. It therefore uses a post-update hook to asynchronously signal to the subgit
that it should import the rep specified. 

The post-update hook isn't strictly necessary, but without it, people using the 
subgit repository will often have a degraded flow - 
they will have to push to force the subgit to import the upstream changes, pull to get those changes, and then push again.

### Commit syncing logic

To keep track of the commits and their synchronization status across the repositories, a sqlite database is used to track how
commits correspond to each other in the two repositories. Two tables - an upstream_to_local table and a
table. 

To translate a commit from one repository to the other, the following logic is used
 * Find the parents of the commit that needs to by synchonized
 * Lookup the corresponding commits in the destination repo
 * Deduplicate the dest repo parent commits (e.g. distinct parents in the source might map to a single commit in the dest)
 * Figure out if there are changes - diff the tree of the new commit against it' first parent, and check for changes in the directory that's being mapped
 * If it's a merge commit or has changes, we build a new commit
 * Otherwise we just update the mapping from the source commit to the parent commit in the destination
 
### Recursion detection
 
Changes are sychronized by pushing changes from a local copy of the corresponding repo - you can configure subigt to bypass the server side hooks of the target repo or not when pushing. If you don't skip the hooks, then a cycle occurs and continuous cycle of synchronization occurs (e.g. you push a change, so the hook tries to push it to the other repo, which has the other hook which tries to push the change to the first repo, etc.). To avoid that, the hook has various settings it can use to detect when the push being made is from the hook in the other repository as part of synchronization. Support exists for no recursion detection, recursion detection based of environment variables (like the `GL_USERNAME` that GitLab sets for hooks it executes) and for using push options.
 
### Locking
 
Basic flock locking is used to prevent the subgit importer from running in parallel. There shouldn't be a big risk of deadlocks because the upstream hook run asynchronously with a double fork (so that the ssh connection can close).
 
 
## The file structure
 
Setup creates four repositories inside of a data directory (usually inside the gitsubdir .git folder), along with an sqlite database and a couple of support files (logging, the hook to be symlinked, and the log file)
 * upstream.git - this is a symlink to the upstream repo.   
 * upstream - this is a working (e.g. not bare) clone of the upstream. It's used for generating commits
 * local.git - this is a bare repo whose content (all save HEAD, hooks/ and the data directory) are symlinked to the corresponding content in the mirror
 * local - the working clone for importing commits
 * map.sqlite - a local only working git repo, used to track the upstream <-> mirror commit mapping.
 
 ### Upstream.git
 
 This repository is a symlink to the upstream repository.
 
 ### Upstream
 
 This repository is used for pushing new from the mirror repo to upstream. 
 
 ### Local.git
 
 This repository is almost identical to the mirror repository. Its content is a little different:
  * The packed_refs, FETCH_HEAD, config, description, info, logs, objects, refs files/directories are symlinks to the same objects in the mirror
  * HEAD is omitted because symlinking it breaks git - instead it is copied since it never changes
  * hooks/ is an empty directory
 
 This repository exists so that when importing commits from the upstream, 
 commits can be pushed to the mirror without triggering another update hook.
 
 While it might be possible to use the plumbing of git to avoid a solution like this, 
 this seemed like the easiest solution.
 
 ### Local
 
 This to push commits into the local.git repository when copying commits from upstream. 
 It clones local.git and not the mirror directly to avoid triggering hooks.
 
 ### Map.sqlite
 
 This database contains a mapping of upstream<->local commit.
 Given a sha, one can look up the corresponding upstream or mirror file which contains the sha it maps to in the other repository.
 All previous mappings are stored with their timestamps, in the hopes that the hook might be able to use that data to get around branch confusion (issue #2)
  
