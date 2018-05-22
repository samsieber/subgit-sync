Logging Setup Needs:
 * Log rotate by day
 * Record what changes are being made (e.g. upstream call or subgit call, & the refs being updated)
 * Record aborts due to being out of date
 * Catch and record backtraces?

Tests Needed:
 * Narrow tests (Notation: A = Applicable, NA = Non-applicable e.g. doesn't need copying to subgit)
    * Running setup against a single branch with multiple commits
    * Subgit: Pushing a new tip pointing to an already imported commit 
    * Subgit: Pushing multiple commits on a single tip
    * Subgit: Pushing multiple tips
    * Subgit: Push tag
    * Subgit: Push signed commit
    * Upstream: Single non-applicable commit is squashed into parent
    * Upstream: Multiple non-applicable commit is squashed into parent
    * Upstream: Merge of NA and A with NA root
    * Upstream: Merge of NA with NA and NA root
    * Upstream: Merge of A with A and NA root
    * Upstream: Merge of NA with A and A root 
    * Upstream: Delete ref
    * Upstream: New ref to existing commit
    * Upstream: New ref to removed commit
    * Upstream: Multiple new refs
    * Upstream: Pushing orphaned commit
    * General: Testing ref-spec filtering
    * Upstream: Push tag (shouldn't replicate)
    
 * Integration Tests:
    * Clone on a big complex repo with multiple branches & root commits 
    * More of them?
 