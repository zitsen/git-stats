use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use clap::Parser;
use git2::{DiffOptions, Repository};

use std::collections::HashMap;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Glob paths
    glob: Vec<String>,
    /// Repository path
    #[arg(short, long, value_name = "PATH")]
    repository: Option<String>,

    /// Module name
    #[arg(short, long)]
    module: Option<String>,

    /// Start time
    #[arg(short, long, value_name = "DATETIME", value_parser = parse_time)]
    since: Option<DateTime<Local>>,

    #[arg(short, long, value_name = "DATETIME", value_parser = parse_time)]
    until: Option<DateTime<Local>>,

    /// Module name
    #[arg(long, default_value = "false")]
    no_bot: bool,
}

fn parse_time(s: &str) -> Result<DateTime<chrono::Local>, String> {
    // 尝试解析日期
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .earliest()
            .ok_or_else(|| format!("invalid time: {s}"));
    }

    // 尝试解析日期加时间
    if let Ok(datetime) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(datetime.with_timezone(&chrono::Local));
    }

    // 尝试解析 RFC3339 格式
    if let Ok(datetime) = DateTime::parse_from_rfc3339(s) {
        return Ok(datetime.with_timezone(&chrono::Local));
    }

    Err("Invalid time format".to_string())
}

struct User {
    email: String,
    time: DateTime<Local>,
    commits: usize,
    added: usize,
    deleted: usize,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let repo = cli.repository.as_deref().unwrap_or(".");
    let repo = Repository::open(repo)?;
    let mut revwalk = repo.revwalk()?;
    // revwalk.push_glob("")?;
    revwalk.push_head()?;

    let mailmap = repo.mailmap()?;

    let mut stats: HashMap<String, User> = HashMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let time: DateTime<Local> = Local.timestamp_opt(commit.time().seconds(), 0).unwrap();

        if let Some(since) = cli.since.as_ref() {
            if time < *since {
                continue;
            }
        }

        if let Some(un) = cli.until.as_ref() {
            if time > *un {
                continue;
            }
        }

        let author = commit.author();
        let can_au = mailmap.resolve_signature(&author)?;
        let author_name = can_au.name().unwrap_or("").to_string();
        let email = can_au.email().unwrap_or("").to_string();

        if !cli.no_bot && author_name.contains("dependabot") {
            continue;
        }

        let tree = commit.tree()?;
        let parent_tree = if commit.parents().len() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut diff_opts = DiffOptions::new();
        for p in &cli.glob {
            diff_opts.pathspec(p);
        }

        let diff =
            repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        let diff_status = diff.stats()?;

        let insertions = diff_status.insertions();
        let deletions = diff_status.deletions();

        let entry = stats.entry(author_name).or_insert(User {
            email,
            time,
            commits: 0,
            added: 0,
            deleted: 0,
        });
        entry.time = time;
        entry.commits += 1; // Increment commit count
        entry.added += insertions;
        entry.deleted += deletions;
    }

    for (
        author,
        User {
            email,
            time,
            commits,
            added,
            deleted,
        },
    ) in stats
    {
        if let Some(m) = cli.module.as_ref() {
            println!(
            "{m}\t{author}\t{email}\t{commits}\t{added}\t{deleted}\t 从 {} 年 {} 月至今，共提交 commit {commits} 个， 新增代码 {added} 行, 删除代码 {deleted} 行",
            time.year(), time.month(),
        );
        } else {
            println!(
            "{author}\t{email}\t{commits}\t{added}\t{deleted}\t 从 {} 年 {} 月至今，共提交 commit {commits} 个， 新增代码 {added} 行, 删除代码 {deleted} 行",
            time.year(), time.month(),
        );
        }
    }

    Ok(())
}
