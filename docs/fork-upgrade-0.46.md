# Fork upgrade to Jujutsu 0.46.0

This fork follows upstream 0.46.0's workspace lifecycle rather than maintaining
parallel worktree creation and cleanup implementations. Git 2.42 or newer is
required; building requires Rust 1.97.1 or newer.

## Workspace command changes

- `jj workspace add` now honors `git.colocate`: automatic colocation requires
  both a colocated current workspace and `git.colocate = true`. Explicit
  `--colocate` and `--no-colocate` override the setting.
- `jj workspace forget NAME` automatically disconnects the associated Git
  worktree, leaving its working-copy files on disk.
- The fork's `workspace forget --cleanup [--force]` flags are removed. Update
  scripts that delete workspaces to use `jj workspace remove NAME`. This is not
  an exact alias: upstream snapshots the working copy before deleting it, and
  refuses to remove the workspace containing the repository. Ignored and
  untracked files excluded from the snapshot are still lost when the directory
  is deleted. Use `workspace forget` when those files must remain.
- `jj git colocation enable` and `disable` now work on child workspaces.

Existing 0.45.1-fork Git worktrees do not need to be recreated. Their `.git`
gitlinks and per-workspace HEAD metadata remain compatible. A custom workspace
name need not match its directory name. The compatibility test for the older
0.44 fork's protobuf HEAD map is also retained.

Upstream adds a workspace-store backend type file during repository loading.
Open older repositories once with write access before making them read-only:
for example, run `jj log --ignore-working-copy` with the new binary while the
repository metadata is writable. A first open of an old, already read-only
repository can fail with a permission error on `workspace_store/type`. This is
separate from the fork's read-only history support for upgraded repositories.

## Remaining fork behavior

- Read-only history commands avoid snapshotting, Git import, and operation
  reconciliation; secure-config fallbacks remain narrowly scoped to access
  failures.
- `ui.graph.text-gap` and `JJ_NO_DEFAULT_COLORS` retain the fork's presentation
  controls, and the README retains the preferred Tau contact.
- Importing Git HEAD still discovers commits made in other linked worktrees.
  Upstream's worktree creation support does not provide that import behavior:
  its HEAD import handles the active workspace only. The fork's additional
  import must not reset the active working copy.
- Disabling colocation in the main workspace still refuses to move the common
  Git directory while linked worktrees exist. Upstream does not repair their
  gitlinks when moving that directory. Disable child colocation or forget the
  children first. The existing `--force` escape hatch remains explicitly unsafe:
  it leaves those Git worktrees broken. Disabling a child workspace does not
  move the common Git directory and is not blocked by this safeguard.

No Nix deployment or package pin is changed by this refresh.
