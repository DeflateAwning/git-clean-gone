use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(name = "git-clean-gone")]
#[command(about = "Clean up local Git branches that have been deleted on the remote", long_about = None)]
struct Args {
    /// Perform a dry run without actually deleting branches
    #[arg(short, long)]
    dry_run: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Ensure we're in a git repository
    ensure_git_repo()?;

    // Fetch and prune
    println!("Fetching and pruning remote branches...");
    git_fetch_prune(args.verbose)?;

    // Find gone branches
    let gone_branches = find_gone_branches(args.verbose)?;

    if gone_branches.is_empty() {
        println!("No gone branches found.");
    } else {
        println!("\nFound {} gone branch(es):", gone_branches.len());
        for branch in &gone_branches {
            println!("  - {branch}");
        }

        if args.dry_run {
            println!(
                "\n[DRY RUN] Would delete {} branch(es)",
                gone_branches.len()
            );
        } else {
            println!("\nDeleting gone branches...");
            delete_branches(&gone_branches, args.verbose)?;
        }
    }

    // Show remaining branches
    println!("\nRemaining branches:");
    show_all_branches()?;

    Ok(())
}

/// Ensures we're inside a git repository
fn ensure_git_repo() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to check if in git repository")?;

    if !output.success() {
        anyhow::bail!("Not in a git repository");
    }

    Ok(())
}

/// Runs `git fetch -ap` to fetch and prune remote branches
fn git_fetch_prune(verbose: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["fetch", "-ap"]);

    if verbose {
        cmd.status().context("Failed to execute git fetch -ap")?;
    } else {
        cmd.stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("Failed to execute git fetch -ap")?;
    }

    Ok(())
}

/// Finds branches marked as "gone" (deleted on remote)
fn find_gone_branches(verbose: bool) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "-vv"])
        .output()
        .context("Failed to execute git branch -vv")?;

    if !output.status.success() {
        anyhow::bail!("git branch -vv failed");
    }

    let stdout =
        String::from_utf8(output.stdout).context("Failed to parse git branch output as UTF-8")?;

    if verbose {
        println!("\nBranch output:");
        println!("{stdout}");
    }

    parse_gone_branches(&stdout)
}

/// Parses the output of `git branch -vv` to find branches with ": gone]"
#[allow(clippy::unnecessary_wraps)]
fn parse_gone_branches(branch_output: &str) -> Result<Vec<String>> {
    let gone_regex = Regex::new(r": gone]").unwrap();
    let current_branch_regex = Regex::new(r"^\*").unwrap();

    let branches: Vec<String> = branch_output
        .lines()
        .filter(|line| gone_regex.is_match(line)) // Contains ": gone]"
        .filter(|line| !current_branch_regex.is_match(line)) // Not current branch
        .filter_map(|line| {
            // Extract branch name (first non-whitespace token, possibly after '*')
            line.split_whitespace().next().map(std::string::ToString::to_string)
        })
        .collect();

    Ok(branches)
}

/// Deletes the specified branches using `git branch -D`
fn delete_branches(branches: &[String], verbose: bool) -> Result<()> {
    if branches.is_empty() {
        return Ok(());
    }

    let mut cmd = Command::new("git");
    cmd.arg("branch").arg("-D");

    for branch in branches {
        cmd.arg(branch);
    }

    let status = if verbose {
        cmd.status()
    } else {
        cmd.stdout(Stdio::inherit()).status()
    }
    .context("Failed to execute git branch -D")?;

    if !status.success() {
        anyhow::bail!("Failed to delete some branches");
    }

    Ok(())
}

/// Shows all branches (local and remote)
fn show_all_branches() -> Result<()> {
    let status = Command::new("git")
        .args(["branch", "-a"])
        .status()
        .context("Failed to execute git branch -a")?;

    if !status.success() {
        anyhow::bail!("git branch -a failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gone_branches_empty() {
        let output = "";
        let branches = parse_gone_branches(output).unwrap();
        assert_eq!(branches.len(), 0);
    }

    #[test]
    fn test_parse_gone_branches_no_gone() {
        let output = r"
  feature-1    abc1234 [origin/feature-1] Some commit
  feature-2    def5678 [origin/feature-2] Another commit
* main         ghi9012 [origin/main] Latest commit
";
        let branches = parse_gone_branches(output).unwrap();
        assert_eq!(branches.len(), 0);
    }

    #[test]
    fn test_parse_gone_branches_with_gone() {
        let output = r"
  feature-1    abc1234 [origin/feature-1: gone] Some commit
  feature-2    def5678 [origin/feature-2] Another commit
  old-feature  ghi9012 [origin/old-feature: gone] Old commit
* main         jkl3456 [origin/main] Latest commit
";
        let branches = parse_gone_branches(output).unwrap();
        assert_eq!(branches.len(), 2);
        assert!(branches.contains(&"feature-1".to_string()));
        assert!(branches.contains(&"old-feature".to_string()));
        assert!(!branches.contains(&"main".to_string()));
    }

    #[test]
    fn test_parse_gone_branches_excludes_current() {
        let output = r"
  feature-1    abc1234 [origin/feature-1: gone] Some commit
* current      def5678 [origin/current: gone] Current branch
";
        let branches = parse_gone_branches(output).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0], "feature-1");
    }

    #[test]
    fn test_parse_gone_branches_with_ahead_behind() {
        let output = r"
  feature-1    abc1234 [origin/feature-1: ahead 2, gone] Some commit
  feature-2    def5678 [origin/feature-2: behind 3] Another commit
  feature-3    ghi9012 [origin/feature-3: ahead 1, behind 2, gone] Mixed commit
";
        let branches = parse_gone_branches(output).unwrap();
        assert_eq!(branches.len(), 2);
        assert!(branches.contains(&"feature-1".to_string()));
        assert!(branches.contains(&"feature-3".to_string()));
    }

    #[test]
    fn test_parse_gone_branches_complex_names() {
        let output = r"
  feature/JIRA-123    abc1234 [origin/feature/JIRA-123: gone] Ticket work
  bugfix/fix-thing    def5678 [origin/bugfix/fix-thing: gone] Bug fix
* main                ghi9012 [origin/main] Latest
";
        let branches = parse_gone_branches(output).unwrap();
        assert_eq!(branches.len(), 2);
        assert!(branches.contains(&"feature/JIRA-123".to_string()));
        assert!(branches.contains(&"bugfix/fix-thing".to_string()));
    }
}
