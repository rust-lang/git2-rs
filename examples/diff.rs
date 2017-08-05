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

extern crate git2;
extern crate docopt;
#[macro_use]
extern crate serde_derive;

use std::str;

use docopt::Docopt;
use git2::{Repository, Error, Object, ObjectType, DiffOptions, Diff};
use git2::{DiffFindOptions, DiffFormat};

#[derive(Deserialize)] #[allow(non_snake_case)]
struct Args {
    arg_from_oid: Option<String>,
    arg_to_oid: Option<String>,
    flag_patch: bool,
    flag_cached: bool,
    flag_nocached: bool,
    flag_name_only: bool,
    flag_name_status: bool,
    flag_raw: bool,
    flag_format: Option<String>,
    flag_color: bool,
    flag_no_color: bool,
    flag_R: bool,
    flag_text: bool,
    flag_ignore_space_at_eol: bool,
    flag_ignore_space_change: bool,
    flag_ignore_all_space: bool,
    flag_ignored: bool,
    flag_untracked: bool,
    flag_patience: bool,
    flag_minimal: bool,
    flag_stat: bool,
    flag_numstat: bool,
    flag_shortstat: bool,
    flag_summary: bool,
    flag_find_renames: Option<u16>,
    flag_find_copies: Option<u16>,
    flag_find_copies_harder: bool,
    flag_break_rewrites: bool,
    flag_unified: Option<u32>,
    flag_inter_hunk_context: Option<u32>,
    flag_abbrev: Option<u16>,
    flag_src_prefix: Option<String>,
    flag_dst_prefix: Option<String>,
    flag_git_dir: Option<String>,
}

const RESET: &'static str = "\u{1b}[m";
const BOLD: &'static str = "\u{1b}[1m";
const RED: &'static str = "\u{1b}[31m";
const GREEN: &'static str = "\u{1b}[32m";
const CYAN: &'static str = "\u{1b}[36m";

#[derive(PartialEq, Eq, Copy, Clone)]
enum Cache { Normal, Only, None }

fn run(args: &Args) -> Result<(), Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = try!(Repository::open(path));

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
    if let Some(amt) = args.flag_unified { opts.context_lines(amt); }
    if let Some(amt) = args.flag_inter_hunk_context { opts.interhunk_lines(amt); }
    if let Some(amt) = args.flag_abbrev { opts.id_abbrev(amt); }
    if let Some(ref s) = args.flag_src_prefix { opts.old_prefix(&s); }
    if let Some(ref s) = args.flag_dst_prefix { opts.new_prefix(&s); }
    if let Some("diff-index") = args.flag_format.as_ref().map(|s| &s[..]) {
        opts.id_abbrev(40);
    }

    // Prepare the diff to inspect
    let t1 = try!(tree_to_treeish(&repo, args.arg_from_oid.as_ref()));
    let t2 = try!(tree_to_treeish(&repo, args.arg_to_oid.as_ref()));
    let head = try!(tree_to_treeish(&repo, Some(&"HEAD".to_string()))).unwrap();
    let mut diff = match (t1, t2, args.cache()) {
        (Some(t1), Some(t2), _) => {
            try!(repo.diff_tree_to_tree(t1.as_tree(), t2.as_tree(),
                                        Some(&mut opts)))
        }
        (t1, None, Cache::None) => {
            let t1 = t1.unwrap_or(head);
            try!(repo.diff_tree_to_workdir(t1.as_tree(), Some(&mut opts)))
        }
        (t1, None, Cache::Only) => {
            let t1 = t1.unwrap_or(head);
            try!(repo.diff_tree_to_index(t1.as_tree(), None, Some(&mut opts)))
        }
        (Some(t1), None, _) => {
            try!(repo.diff_tree_to_workdir_with_index(t1.as_tree(),
                                                      Some(&mut opts)))
        }
        (None, None, _) => {
            try!(repo.diff_index_to_workdir(None, Some(&mut opts)))
        }
        (None, Some(_), _) => unreachable!(),
    };

    // Apply rename and copy detection if requested
    if args.flag_break_rewrites || args.flag_find_copies_harder ||
       args.flag_find_renames.is_some() || args.flag_find_copies.is_some()
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
        try!(diff.find_similar(Some(&mut opts)));
    }

    // Generate simple output
    let stats = args.flag_stat | args.flag_numstat | args.flag_shortstat |
                args.flag_summary;
    if stats {
        try!(print_stats(&diff, args));
    }
    if args.flag_patch || !stats {
        if args.color() { print!("{}", RESET); }
        let mut last_color = None;
        try!(diff.print(args.diff_format(), |_delta, _hunk, line| {
            if args.color() {
                let next = match line.origin() {
                    '+' => Some(GREEN),
                    '-' => Some(RED),
                    '>' => Some(GREEN),
                    '<' => Some(RED),
                    'F' => Some(BOLD),
                    'H' => Some(CYAN),
                    _ => None
                };
                if args.color() && next != last_color {
                    if last_color == Some(BOLD) || next == Some(BOLD) {
                        print!("{}", RESET);
                    }
                    print!("{}", next.unwrap_or(RESET));
                    last_color = next;
                }
            }

            match line.origin() {
                '+' | '-' | ' ' => print!("{}", line.origin()),
                _ => {}
            }
            print!("{}", str::from_utf8(line.content()).unwrap());
            true
        }));
        if args.color() { print!("{}", RESET); }
    }

    Ok(())
}

fn print_stats(diff: &Diff, args: &Args) -> Result<(), Error> {
    let stats = try!(diff.stats());
    let mut format = git2::DIFF_STATS_NONE;
    if args.flag_stat {
        format |= git2::DIFF_STATS_FULL;
    }
    if args.flag_shortstat {
        format |= git2::DIFF_STATS_SHORT;
    }
    if args.flag_numstat {
        format |= git2::DIFF_STATS_NUMBER;
    }
    if args.flag_summary {
        format |= git2::DIFF_STATS_INCLUDE_SUMMARY;
    }
    let buf = try!(stats.to_buf(format, 80));
    print!("{}", str::from_utf8(&*buf).unwrap());
    Ok(())
}

fn tree_to_treeish<'a>(repo: &'a Repository, arg: Option<&String>)
                       -> Result<Option<Object<'a>>, Error> {
    let arg = match arg { Some(s) => s, None => return Ok(None) };
    let obj = try!(repo.revparse_single(arg));
    let tree = try!(obj.peel(ObjectType::Tree));
    Ok(Some(tree))
}

impl Args {
    fn cache(&self) -> Cache {
        if self.flag_cached {Cache::Only}
        else if self.flag_nocached {Cache::None}
        else {Cache::Normal}
    }
    fn color(&self) -> bool { self.flag_color && !self.flag_no_color }
    fn diff_format(&self) -> DiffFormat {
        if self.flag_patch {DiffFormat::Patch}
        else if self.flag_name_only {DiffFormat::NameOnly}
        else if self.flag_name_status {DiffFormat::NameStatus}
        else if self.flag_raw {DiffFormat::Raw}
        else {
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
    const USAGE: &'static str = "
usage: diff [options] [<from-oid> [<to-oid>]]

Options:
    -p, --patch                 show output in patch format
    --cached                    use staged changes as diff
    --nocached                  do not use staged changes
    --name-only                 show only names of changed files
    --name-status               show only names and status changes
    --raw                       generate the raw format
    --format=<format>           specify format for stat summary
    --color                     use color output
    --no-color                  never use color output
    -R                          swap two inputs
    -a, --text                  treat all files as text
    --ignore-space-at-eol       ignore changes in whitespace at EOL
    -b, --ignore-space-change   ignore changes in amount of whitespace
    -w, --ignore-all-space      ignore whitespace when comparing lines
    --ignored                   show ignored files as well
    --untracked                 show untracked files
    --patience                  generate diff using the patience algorithm
    --minimal                   spend extra time to find smallest diff
    --stat                      generate a diffstat
    --numstat                   similar to --stat, but more machine friendly
    --shortstat                 only output last line of --stat
    --summary                   output condensed summary of header info
    -M, --find-renames <n>      set threshold for findind renames (default 50)
    -C, --find-copies <n>       set threshold for finding copies (default 50)
    --find-copies-harder        inspect unmodified files for sources of copies
    -B, --break-rewrites        break complete rewrite changes into pairs
    -U, --unified <n>           lints of context to show
    --inter-hunk-context <n>    maximum lines of change between hunks
    --abbrev <n>                length to abbreviate commits to
    --src-prefix <s>            show given source prefix instead of 'a/'
    --dst-prefix <s>            show given destinction prefix instead of 'b/'
    --git-dir <path>            path to git repository to use
    -h, --help                  show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.deserialize())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
