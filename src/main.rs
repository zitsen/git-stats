use git2::{Repository, DiffOptions};
use clap::Parser;

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Glob paths
    glob: Vec<PathBuf>,
    /// Repository path
    #[arg(short, long, value_name = "PATH")]
    repository: Option<String>,

    /// Start time
    #[arg(short, long, value_name = "DATETIME")]
    since: Option<String>,

    #[arg(short, long, value_name = "DATETIME")]
    until: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let repo = cli.repository.as_deref().unwrap_or(".");
    let repo = Repository::open(repo)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_glob("taosx-core/**/*.json")?;
    //revwalk.push_head()?;

    let mailmap = repo.mailmap()?;

    let mut stats: HashMap<String, (usize, usize, usize)> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        let author = commit.author();
        let author_name = mailmap.resolve_signature(&author)?.name().unwrap_or("").to_string();

        let tree = commit.tree()?;
        let parent_tree = if commit.parents().len() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut diff_opts = DiffOptions::new();

        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        let diff_status = diff.stats()?;

        let insertions = diff_status.insertions();
        let deletions = diff_status.deletions();

        let entry = stats.entry(author_name).or_insert((0, 0, 0));
        entry.0 += 1; // Increment commit count
        entry.1 += insertions;
        entry.2 += deletions;
    }

    for (author, (commits, added, deleted)) in stats {
        println!("{}: {} commits, {} lines added, {} lines deleted", author, commits, added, deleted);
    }

    Ok(())
}
