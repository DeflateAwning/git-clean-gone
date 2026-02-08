# git-clean-gone

Run `git clean-gone` to delete any GitHub branches that've been deleted on the remote (e.g., after merging PRs)

## Description

A Rust CLI tool to clean up local Git branches that have been deleted on the remote.

## Features

- Fetches and prunes remote branches
- Identifies local branches whose remote counterparts have been deleted
- Safely deletes those branches (excludes the current branch)
- Supports dry-run mode to preview what would be deleted
- Verbose mode for debugging

## Installation

```bash
cargo install git-clean-gone

# Or, from source:
git clone https://github.com/DeflateAwning/git-clean-gone
cd git-clean-gone
cargo install --path .
```

## Usage

Basic usage (will delete branches):

```bash
git-clean-gone
git clean-gone
```

Dry run (preview without deleting):

```bash
git-clean-gone --dry-run
git clean-gone --dry-run
```

Verbose output:

```bash
git-clean-gone --verbose
```

Combined:

```bash
git-clean-gone --dry-run --verbose
```

## How It Works

1. Runs `git fetch -ap` to fetch all remotes and prune deleted remote branches
2. Runs `git branch -vv` to list local branches with their tracking information
3. Parses the output to find branches marked as `: gone]` (remote deleted)
4. Filters out the current branch (marked with `*`)
5. Deletes the gone branches using `git branch -D`
6. Displays all remaining branches

## Testing

Run the unit tests:

```bash
cargo test
```

## Alternatives

Add the following fish function to your `~/.config/fish/my_alias.fish`:


```fish
function git_clean_gone_branches
    git fetch -ap
    git branch -vv | grep ': gone]' | grep -v "\*" | awk '{ print $1; }' | xargs -r git branch -D
    echo 'Remaining branches:'
    git branch -a
end
```

Why I prefer my awesome super-extra Rust package:
* Works on all shells (fish, bash, zsh).
* Easier install (unless you're already using cool dotfile management)
