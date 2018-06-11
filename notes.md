Logging Setup Needs:
 * Log rotate by day
 * Record what changes are being made (e.g. upstream call or subgit call, & the refs being updated)
 * Record aborts due to being out of date
 * Catch and record backtraces?
 
Things to do:
 * Filter refs for pushes
 * Add option for easy overwriting of hook
 * Disable GC in working repos
 * Remove old refs during sync-all

Tests Needed:
 * Narrow tests (Notation: A = Applicable, NA = Non-applicable e.g. doesn't need copying to subgit)
    * Upstream (import & upstream push):
        * Single non-applicable commit is squashed into parent
        * Multiple non-applicable commit is squashed into parent
        * Merge of NA and A with NA root
        * Merge of NA with NA and NA root
        * Merge of A with A and NA root
        * Merge of NA with A and A root 
        * New ref to existing commit
        * New ref to removed commit
        * Multiple new refs
    * General (upstream & downstream)
        * Pushing:
            * (New|Existing sha) x (New|Existing Ref)
            * Force delete
            * Orphaned commit
        * Refs
            * Works only on refs/heads/*
            * ignore tags
            * Signed commits aren't resigned
            * What about refs pointing to other refs - are these only tags?
    
 * General Tests:
    * Clone on a big complex repo with multiple branches & root commits & make sure it looks okay
    * Benchmark tests?
 