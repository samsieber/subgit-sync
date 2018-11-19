# SubGit RS [WIP]

Subgit lets you publish a subdirectory in a git repository as it's own synchronized git repository
without requiring the use of git submodules or git subtrees. It uses a couple of server-side hooks instead.

Under the hood, it is a single binary three distinct functions:
  * It can run the setup and initial synchronization, copying itself to into the subgit and upstream repositories in the appropriate locations
  * Can act as the update hook in the subgit repository
  * Can act as the post-receive hook in the upstream hook
  
And example invocation of the program would be `subgit-rs some/upstream/repo.git target/subgit.git dir/to/republish`, 
which would setup the `target/subgit.git` to be a synchronized copy of the `dir/to/republish` folder from the `some/upstream/repo.git` repository.

This project is implemented using the Rust programming language, and it compiles down to a single binary. It uses the 
libgit2 rust bindings and a few other rust dependencies (logging, command line option parsing, etc).

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

To keep track of the commits and their synchronization status across the repositories, a file database is used track how
commits correspond to each other in the two repositories. Two folders are maintained - an upstream_to_local folder and a
local_to_upstream. 

To translate a commit from one repository to the other, the following logic is used
 * Find the parents of the commit that needs to by synchonized
 * Lookup the corresponding commits in the destination repo
 * Deduplicate the dest repo parent commits (e.g. distinct parents in the source might map to a single commit in the dest)
 * Figure out if there are changes - diff the tree of the new commit against it' first parent, and check for changes in the directory that's being mapped
 * If it's a merge commit or has changes, we build a new commit
 * Otherwise we just update the mapping from the source commit to the parent commit in the destination
 
 
 ## The fine details
 
 Setup creates five repositories inside of a data directory (usually inside the gitsubdir .git folder).
  * upstream.git - this is a symlink to the upstream repo. It's used for generating commits
  * upstream - this is a working (e.g. not bare) clone of the upstream
  * local.git - this is a bare repo whose content (all save HEAD, hooks/ and the data directory) are symlinked to the corresponding content in the mirror
  * local - the working clone of local.git (not the mirror). 
  We point at local.git instead of the mirror so that we don't trigger hooks when pushing.
  * map - a local only working git repo, used to track the upstream <-> mirror commit mapping.
 
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
 
 ### Map
 
 This repo is locally created and contains a mapping of upstream<->local commit.
 The mapping is stored on disk as a key-value store. 
 Given a sha, one can look up the corresponding upstream or mirror file which contains the sha it maps to in the other repository.
 It's a repo so that if we run into errors while copying commits, we can reset the state back to the HEAD.
  