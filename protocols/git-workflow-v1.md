# NEMESIS Local Git Workflow v1

Git actions are typed operations evaluated against explicit mission authority. The local desktop profile authorizes observation, isolated branch creation, and exact-path local commits. Push, merge, remote deletion, unrelated branch deletion, and unrelated worktree deletion refuse.

A lane commit requires:

- a worktree belonging to the canonical repository's common Git directory;
- an exact `nemesis/` lane branch;
- nonempty message and commit authority;
- a unique list of relative normal-component paths excluding `.git`;
- `git add -- <exact paths>` rather than ambient staging;
- staged path equality with the request;
- `git diff --cached --check` success;
- binary diff SHA-256 receipt;
- unchanged default-branch head before and after commit.

Untracked unrelated files remain uncommitted. CI observations are accepted only when the check name is bounded, conclusion is exactly `success`, evidence digest is nonzero, and source digest equals current source.

```sh
./scripts/test_git_workflows.sh
```

This local workflow prepares commits and source-bound CI observations. It performs no push, merge, PR submission, issue mutation, or release action in this mission.
