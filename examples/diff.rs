/*
 * libgit2 "diff" example - shows how to use the diff API
 *
 * Written by the libgit2 contributors
 *
 * To the extent possible under law, the author(s) have dedicated all copyright
 * and related and neighboring rights to this software to the public domain
 * worldwide. This software is distributed without any warranty.
 *
 * You should have received a copy of the CC0 Public Domain Dedication along
 * with this software. If not, see
 * <http://creativecommons.org/publicdomain/zero/1.0/>.
 */

#![deny(warnings)]

use clap::Parser;
use git2::{Blob, Diff, DiffOptions, Error, Object, ObjectType, Oid, Repository};
use git2::{DiffDelta, DiffFindOptions, DiffFormat, DiffHunk, DiffLine};
use std::str;

#[derive(Parser)]
#[allow(non_snake_case)]
struct Args {
    #[structopt(name = "from_oid")]
    arg_from_oid: Option<String>,
    #[structopt(name = "to_oid")]
    arg_to_oid: Option<String>,
    #[structopt(name = "blobs", long)]
    /// treat from_oid and to_oid as blob ids
    flag_blobs: bool,
    #[structopt(name = "patch", short, long)]
    /// show output in patch format
    flag_patch: bool,
    #[structopt(name = "cached", long)]
    /// use staged changes as diff
    flag_cached: bool,
    #[structopt(name = "nocached", long)]
    /// do not use staged changes
    flag_nocached: bool,
    #[structopt(name = "name-only", long)]
    /// show only names of changed files
    flag_name_only: bool,
    #[structopt(name = "name-status", long)]
    /// show only names and status changes
    flag_name_status: bool,
    #[structopt(name = "raw", long)]
    /// generate the raw format
    flag_raw: bool,
    #[structopt(name = "format", long)]
    /// specify format for stat summary
    flag_format: Option<String>,
    #[structopt(name = "color", long)]
    /// use color output
    flag_color: bool,
    #[structopt(name = "no-color", long)]
    /// never use color output
    flag_no_color: bool,
    #[structopt(short = 'R')]
    /// swap two inputs
    flag_R: bool,
    #[structopt(name = "text", short = 'a', long)]
    /// treat all files as text
    flag_text: bool,
    #[structopt(name = "ignore-space-at-eol", long)]
    /// ignore changes in whitespace at EOL
    flag_ignore_space_at_eol: bool,
    #[structopt(name = "ignore-space-change", short = 'b', long)]
    /// ignore changes in amount of whitespace
    flag_ignore_space_change: bool,
    #[structopt(name = "ignore-all-space", short = 'w', long)]
    /// ignore whitespace when comparing lines
    flag_ignore_all_space: bool,
    #[structopt(name = "ignored", long)]
    /// show untracked files
    flag_ignored: bool,
    #[structopt(name = "untracked", long)]
    /// generate diff using the patience algorithm
    flag_untracked: bool,
    #[structopt(name = "patience", long)]
    /// show ignored files as well
    flag_patience: bool,
    #[structopt(name = "minimal", long)]
    /// spend extra time to find smallest diff
    flag_minimal: bool,
    #[structopt(name = "stat", long)]
    /// generate a diffstat
    flag_stat: bool,
    #[structopt(name = "numstat", long)]
    /// similar to --stat, but more machine friendly
    flag_numstat: bool,
    #[structopt(name = "shortstat", long)]
    /// only output last line of --stat
    flag_shortstat: bool,
    #[structopt(name = "summary", long)]
    /// output condensed summary of header info
    flag_summary: bool,
    #[structopt(name = "find-renames", short = 'M', long)]
    /// set threshold for finding renames (default 50)
    flag_find_renames: Option<u16>,
    #[structopt(name = "find-copies", short = 'C', long)]
    /// set threshold for finding copies (default 50)
    flag_find_copies: Option<u16>,
    #[structopt(name = "find-copies-harder", long)]
    /// inspect unmodified files for sources of copies
    flag_find_copies_harder: bool,
    #[structopt(name = "break_rewrites", short = 'B', long)]
    /// break complete rewrite changes into pairs
    flag_break_rewrites: bool,
    #[structopt(name = "unified", short = 'U', long)]
    /// lints of context to show
    flag_unified: Option<u32>,
    #[structopt(name = "inter-hunk-context", long)]
    /// maximum lines of change between hunks
    flag_inter_hunk_context: Option<u32>,
    #[structopt(name = "abbrev", long)]
    /// length to abbreviate commits to
    flag_abbrev: Option<u16>,
    #[structopt(name = "src-prefix", long)]
    /// show given source prefix instead of 'a/'
    flag_src_prefix: Option<String>,
    #[structopt(name = "dst-prefix", long)]
    /// show given destination prefix instead of 'b/'
    flag_dst_prefix: Option<String>,
    #[structopt(name = "path", long = "git-dir")]
    /// path to git repository to use
    flag_git_dir: Option<String>,
}

const RESET: &str = "\u{1b}[m";
const BOLD: &str = "\u{1b}[1m";
const RED: &str = "\u{1b}[31m";
const GREEN: &str = "\u{1b}[32m";
const CYAN: &str = "\u{1b}[36m";

#[derive(PartialEq, Eq, Copy, Clone)]
enum Cache {
    Normal,
    Only,
    None,
}

fn line_color(line: &DiffLine) -> Option<&'static str> {
    match line.origin() {
        '+' => Some(GREEN),
        '-' => Some(RED),
        '>' => Some(GREEN),
        '<' => Some(RED),
        'F' => Some(BOLD),
        'H' => Some(CYAN),
        _ => None,
    }
}

fn print_diff_line(
    _delta: DiffDelta,
    _hunk: Option<DiffHunk>,
    line: DiffLine,
    args: &Args,
) -> bool {
    if args.color() {
        print!("{}", RESET);
        if let Some(color) = line_color(&line) {
            print!("{}", color);
        }
    }
    match line.origin() {
        '+' | '-' | ' ' => print!("{}", line.origin()),
        _ => {}
    }
    print!("{}", str::from_utf8(line.content()).unwrap());
    true
}

fn run(args: &Args) -> Result<(), Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = Repository::open(path)?;

    // Prepare our diff options based on the arguments given
    let mut opts = DiffOptions::new();
    opts.reverse(args.flag_R)
        .force_text(args.flag_text)
        .ignore_whitespace_eol(args.flag_ignore_space_at_eol)
        .ignore_whitespace_change(args.flag_ignore_space_change)
        .ignore_whitespace(args.flag_ignore_all_space)
        .include_ignored(args.flag_ignored)
        .include_untracked(args.flag_untracked)
        .patience(args.flag_patience)
        .minimal(args.flag_minimal);
    if let Some(amt) = args.flag_unified {
        opts.context_lines(amt);
    }
    if let Some(amt) = args.flag_inter_hunk_context {
        opts.interhunk_lines(amt);
    }
    if let Some(amt) = args.flag_abbrev {
        opts.id_abbrev(amt);
    }
    if let Some(ref s) = args.flag_src_prefix {
        opts.old_prefix(&s);
    }
    if let Some(ref s) = args.flag_dst_prefix {
        opts.new_prefix(&s);
    }
    if let Some("diff-index") = args.flag_format.as_ref().map(|s| &s[..]) {
        opts.id_abbrev(40);
    }

    if args.flag_blobs {
        let b1 = resolve_blob(&repo, args.arg_from_oid.as_ref())?;
        let b2 = resolve_blob(&repo, args.arg_to_oid.as_ref())?;
        repo.diff_blobs(
            b1.as_ref(),
            None,
            b2.as_ref(),
            None,
            Some(&mut opts),
            None,
            None,
            None,
            Some(&mut |d, h, l| print_diff_line(d, h, l, args)),
        )?;
        if args.color() {
            print!("{}", RESET);
        }
        return Ok(());
    }

    // Prepare the diff to inspect
    let t1 = tree_to_treeish(&repo, args.arg_from_oid.as_ref())?;
    let t2 = tree_to_treeish(&repo, args.arg_to_oid.as_ref())?;
    let head = tree_to_treeish(&repo, Some(&"HEAD".to_string()))?.unwrap();
    let mut diff = match (t1, t2, args.cache()) {
        (Some(t1), Some(t2), _) => {
            repo.diff_tree_to_tree(t1.as_tree(), t2.as_tree(), Some(&mut opts))?
        }
        (t1, None, Cache::None) => {
            let t1 = t1.unwrap_or(head);
            repo.diff_tree_to_workdir(t1.as_tree(), Some(&mut opts))?
        }
        (t1, None, Cache::Only) => {
            let t1 = t1.unwrap_or(head);
            repo.diff_tree_to_index(t1.as_tree(), None, Some(&mut opts))?
        }
        (Some(t1), None, _) => {
            repo.diff_tree_to_workdir_with_index(t1.as_tree(), Some(&mut opts))?
        }
        (None, None, _) => repo.diff_index_to_workdir(None, Some(&mut opts))?,
        (None, Some(_), _) => unreachable!(),
    };

    // Apply rename and copy detection if requested
    if args.flag_break_rewrites
        || args.flag_find_copies_harder
        || args.flag_find_renames.is_some()
        || args.flag_find_copies.is_some()
    {
        let mut opts = DiffFindOptions::new();
        if let Some(t) = args.flag_find_renames {
            opts.rename_threshold(t);
            opts.renames(true);
        }
        if let Some(t) = args.flag_find_copies {
            opts.copy_threshold(t);
            opts.copies(true);
        }
        opts.copies_from_unmodified(args.flag_find_copies_harder)
            .rewrites(args.flag_break_rewrites);
        diff.find_similar(Some(&mut opts))?;
    }

    // Generate simple output
    let stats = args.flag_stat | args.flag_numstat | args.flag_shortstat | args.flag_summary;
    if stats {
        print_stats(&diff, args)?;
    }
    if args.flag_patch || !stats {
        diff.print(args.diff_format(), |d, h, l| print_diff_line(d, h, l, args))?;
        if args.color() {
            print!("{}", RESET);
        }
    }

    Ok(())
}

fn print_stats(diff: &Diff, args: &Args) -> Result<(), Error> {
    let stats = diff.stats()?;
    let mut format = git2::DiffStatsFormat::NONE;
    if args.flag_stat {
        format |= git2::DiffStatsFormat::FULL;
    }
    if args.flag_shortstat {
        format |= git2::DiffStatsFormat::SHORT;
    }
    if args.flag_numstat {
        format |= git2::DiffStatsFormat::NUMBER;
    }
    if args.flag_summary {
        format |= git2::DiffStatsFormat::INCLUDE_SUMMARY;
    }
    let buf = stats.to_buf(format, 80)?;
    print!("{}", str::from_utf8(&*buf).unwrap());
    Ok(())
}

fn tree_to_treeish<'a>(
    repo: &'a Repository,
    arg: Option<&String>,
) -> Result<Option<Object<'a>>, Error> {
    let arg = match arg {
        Some(s) => s,
        None => return Ok(None),
    };
    let obj = repo.revparse_single(arg)?;
    let tree = obj.peel(ObjectType::Tree)?;
    Ok(Some(tree))
}

fn resolve_blob<'a>(repo: &'a Repository, arg: Option<&String>) -> Result<Option<Blob<'a>>, Error> {
    let arg = match arg {
        Some(s) => Oid::from_str(s)?,
        None => return Ok(None),
    };
    repo.find_blob(arg).map(|b| Some(b))
}

impl Args {
    fn cache(&self) -> Cache {
        if self.flag_cached {
            Cache::Only
        } else if self.flag_nocached {
            Cache::None
        } else {
            Cache::Normal
        }
    }
    fn color(&self) -> bool {
        self.flag_color && !self.flag_no_color
    }
    fn diff_format(&self) -> DiffFormat {
        if self.flag_patch {
            DiffFormat::Patch
        } else if self.flag_name_only {
            DiffFormat::NameOnly
        } else if self.flag_name_status {
            DiffFormat::NameStatus
        } else if self.flag_raw {
            DiffFormat::Raw
        } else {
            match self.flag_format.as_ref().map(|s| &s[..]) {
                Some("name") => DiffFormat::NameOnly,
                Some("name-status") => DiffFormat::NameStatus,
                Some("raw") => DiffFormat::Raw,
                Some("diff-index") => DiffFormat::Raw,
                _ => DiffFormat::Patch,
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
