/*
 * libgit2 "status" example - shows how to use the status APIs
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
extern crate rustc_serialize;

use std::str;
use std::time::Duration;
use docopt::Docopt;
use git2::{Repository, Error, StatusOptions, ErrorCode, SubmoduleIgnore};

#[derive(RustcDecodable)]
struct Args {
    arg_spec: Vec<String>,
    flag_short: bool,
    flag_porcelain: bool,
    flag_branch: bool,
    flag_z: bool,
    flag_ignored: bool,
    flag_untracked_files: Option<String>,
    flag_ignore_submodules: Option<String>,
    flag_git_dir: Option<String>,
    flag_repeat: bool,
    flag_list_submodules: bool,
}

#[derive(Eq, PartialEq)]
enum Format { Long, Short, Porcelain }

fn run(args: &Args) -> Result<(), Error> {
    let path = args.flag_git_dir.clone().unwrap_or_else(|| ".".to_string());
    let repo = try!(Repository::open(&path));
    if repo.is_bare() {
        return Err(Error::from_str("cannot report status on bare repository"))
    }

    let mut opts = StatusOptions::new();
    opts.include_ignored(args.flag_ignored);
    match args.flag_untracked_files.as_ref().map(|s| &s[..]) {
        Some("no") => { opts.include_untracked(false); }
        Some("normal") => { opts.include_untracked(true); }
        Some("all") => {
            opts.include_untracked(true).recurse_untracked_dirs(true);
        }
        Some(_) => return Err(Error::from_str("invalid untracked-files value")),
        None => {}
    }
    match args.flag_ignore_submodules.as_ref().map(|s| &s[..]) {
        Some("all") => { opts.exclude_submodules(true); }
        Some(_) => return Err(Error::from_str("invalid ignore-submodules value")),
        None => {}
    }
    opts.include_untracked(!args.flag_ignored);
    for spec in &args.arg_spec {
        opts.pathspec(spec);
    }

    loop {
        if args.flag_repeat {
            println!("\u{1b}[H\u{1b}[2J");
        }

        let statuses = try!(repo.statuses(Some(&mut opts)));

        if args.flag_branch {
            try!(show_branch(&repo, &args.format()));
        }
        if args.flag_list_submodules {
            try!(print_submodules(&repo));
        }

        if args.format() == Format::Long {
            print_long(&statuses);
        } else {
            print_short(&repo, &statuses);
        }

        if args.flag_repeat {
            std::thread::sleep(Duration::new(10, 0));
        } else {
            return Ok(())
        }
    }
}

fn show_branch(repo: &Repository, format: &Format) -> Result<(), Error> {
    let head = match repo.head() {
        Ok(head) => Some(head),
        Err(ref e) if e.code() == ErrorCode::UnbornBranch ||
                      e.code() == ErrorCode::NotFound => None,
        Err(e) => return Err(e),
    };
    let head = head.as_ref().and_then(|h| h.shorthand());

    if format == &Format::Long {
        println!("# On branch {}",
                 head.unwrap_or("Not currently on any branch"));
    } else {
        println!("## {}", head.unwrap_or("HEAD (no branch)"));
    }
    Ok(())
}

fn print_submodules(repo: &Repository) -> Result<(), Error> {
    let modules = try!(repo.submodules());
    println!("# Submodules");
    for sm in &modules {
        println!("# - submodule '{}' at {}", sm.name().unwrap(),
                 sm.path().display());
    }
    Ok(())
}

// This function print out an output similar to git's status command in long
// form, including the command-line hints.
fn print_long(statuses: &git2::Statuses) {
    let mut header = false;
    let mut rm_in_workdir = false;
    let mut changes_in_index = false;
    let mut changed_in_workdir = false;

    // Print index changes
    for entry in statuses.iter().filter(|e| e.status() != git2::STATUS_CURRENT) {
        if entry.status().contains(git2::STATUS_WT_DELETED) {
            rm_in_workdir = true;
        }
        let istatus = match entry.status() {
            s if s.contains(git2::STATUS_INDEX_NEW) => "new file: ",
            s if s.contains(git2::STATUS_INDEX_MODIFIED) => "modified: ",
            s if s.contains(git2::STATUS_INDEX_DELETED) => "deleted: ",
            s if s.contains(git2::STATUS_INDEX_RENAMED) => "renamed: ",
            s if s.contains(git2::STATUS_INDEX_TYPECHANGE) => "typechange:",
            _ => continue,
        };
        if !header {
            println!("\
# Changes to be committed:
#   (use \"git reset HEAD <file>...\" to unstage)
#");
            header = true;
        }

        let old_path = entry.head_to_index().unwrap().old_file().path();
        let new_path = entry.head_to_index().unwrap().new_file().path();
        match (old_path, new_path) {
            (Some(old), Some(new)) if old != new => {
                println!("#\t{}  {} -> {}", istatus, old.display(),
                         new.display());
            }
            (old, new) => {
                println!("#\t{}  {}", istatus, old.or(new).unwrap().display());
            }
        }
    }

    if header {
        changes_in_index = true;
        println!("#");
    }
    header = false;

    // Print workdir changes to tracked files
    for entry in statuses.iter() {
        // With `STATUS_OPT_INCLUDE_UNMODIFIED` (not used in this example)
        // `index_to_workdir` may not be `None` even if there are no differences,
        // in which case it will be a `Delta::Unmodified`.
        if entry.status() == git2::STATUS_CURRENT ||
           entry.index_to_workdir().is_none() {
            continue
        }

        let istatus = match entry.status() {
            s if s.contains(git2::STATUS_WT_MODIFIED) => "modified: ",
            s if s.contains(git2::STATUS_WT_DELETED) => "deleted: ",
            s if s.contains(git2::STATUS_WT_RENAMED) => "renamed: ",
            s if s.contains(git2::STATUS_WT_TYPECHANGE) => "typechange:",
            _ => continue,
        };

        if !header {
            println!("\
# Changes not staged for commit:
#   (use \"git add{} <file>...\" to update what will be committed)
#   (use \"git checkout -- <file>...\" to discard changes in working directory)
#\
                ", if rm_in_workdir {"/rm"} else {""});
            header = true;
        }

        let old_path = entry.index_to_workdir().unwrap().old_file().path();
        let new_path = entry.index_to_workdir().unwrap().new_file().path();
        match (old_path, new_path) {
            (Some(old), Some(new)) if old != new => {
                println!("#\t{}  {} -> {}", istatus, old.display(),
                         new.display());
            }
            (old, new) => {
                println!("#\t{}  {}", istatus, old.or(new).unwrap().display());
            }
        }
    }

    if header {
        changed_in_workdir = true;
        println!("#");
    }
    header = false;

    // Print untracked files
    for entry in statuses.iter().filter(|e| e.status() == git2::STATUS_WT_NEW) {
        if !header {
            println!("\
# Untracked files
#   (use \"git add <file>...\" to include in what will be committed)
#");
            header = true;
        }
        let file = entry.index_to_workdir().unwrap().old_file().path().unwrap();
        println!("#\t{}", file.display());
    }
    header = false;

    // Print ignored files
    for entry in statuses.iter().filter(|e| e.status() == git2::STATUS_IGNORED) {
        if !header {
            println!("\
# Ignored files
#   (use \"git add -f <file>...\" to include in what will be committed)
#");
            header = true;
        }
        let file = entry.index_to_workdir().unwrap().old_file().path().unwrap();
        println!("#\t{}", file.display());
    }

    if !changes_in_index && changed_in_workdir {
        println!("no changes added to commit (use \"git add\" and/or \
                  \"git commit -a\")");
    }
}

// This version of the output prefixes each path with two status columns and
// shows submodule status information.
fn print_short(repo: &Repository, statuses: &git2::Statuses) {
    for entry in statuses.iter().filter(|e| e.status() != git2::STATUS_CURRENT) {
        let mut istatus = match entry.status() {
            s if s.contains(git2::STATUS_INDEX_NEW) => 'A',
            s if s.contains(git2::STATUS_INDEX_MODIFIED) => 'M',
            s if s.contains(git2::STATUS_INDEX_DELETED) => 'D',
            s if s.contains(git2::STATUS_INDEX_RENAMED) => 'R',
            s if s.contains(git2::STATUS_INDEX_TYPECHANGE) => 'T',
            _ => ' ',
        };
        let mut wstatus = match entry.status() {
            s if s.contains(git2::STATUS_WT_NEW) => {
                if istatus == ' ' { istatus = '?'; } '?'
            }
            s if s.contains(git2::STATUS_WT_MODIFIED) => 'M',
            s if s.contains(git2::STATUS_WT_DELETED) => 'D',
            s if s.contains(git2::STATUS_WT_RENAMED) => 'R',
            s if s.contains(git2::STATUS_WT_TYPECHANGE) => 'T',
            _ => ' ',
        };

        if entry.status().contains(git2::STATUS_IGNORED) {
            istatus = '!';
            wstatus = '!';
        }
        if istatus == '?' && wstatus == '?' { continue }
        let mut extra = "";

        // A commit in a tree is how submodules are stored, so let's go take a
        // look at its status.
        //
        // TODO: check for GIT_FILEMODE_COMMIT
        let status = entry.index_to_workdir().and_then(|diff| {
            let ignore = SubmoduleIgnore::Unspecified;
            diff.new_file().path_bytes()
                .and_then(|s| str::from_utf8(s).ok())
                .and_then(|name| repo.submodule_status(name, ignore).ok())
        });
        if let Some(status) = status {
            if status.contains(git2::SUBMODULE_STATUS_WD_MODIFIED) {
                extra = " (new commits)";
            } else if status.contains(git2::SUBMODULE_STATUS_WD_INDEX_MODIFIED) || status.contains(git2::SUBMODULE_STATUS_WD_WD_MODIFIED) {
                extra = " (modified content)";
            } else if status.contains(git2::SUBMODULE_STATUS_WD_UNTRACKED) {
                extra = " (untracked content)";
            }
        }

        let (mut a, mut b, mut c) = (None, None, None);
        if let Some(diff) = entry.head_to_index() {
            a = diff.old_file().path();
            b = diff.new_file().path();
        }
        if let Some(diff) = entry.index_to_workdir() {
            a = a.or_else(|| diff.old_file().path());
            b = b.or_else(|| diff.old_file().path());
            c = diff.new_file().path();
        }

        match (istatus, wstatus) {
            ('R', 'R') => println!("RR {} {} {}{}", a.unwrap().display(),
                                   b.unwrap().display(), c.unwrap().display(),
                                   extra),
            ('R', w) => println!("R{} {} {}{}", w, a.unwrap().display(),
                                 b.unwrap().display(), extra),
            (i, 'R') => println!("{}R {} {}{}", i, a.unwrap().display(),
                                 c.unwrap().display(), extra),
            (i, w) => println!("{}{} {}{}", i, w, a.unwrap().display(), extra),
        }
    }

    for entry in statuses.iter().filter(|e| e.status() == git2::STATUS_WT_NEW) {
        println!("?? {}", entry.index_to_workdir().unwrap().old_file()
                               .path().unwrap().display());
    }
}

impl Args {
    fn format(&self) -> Format {
        if self.flag_short { Format::Short }
        else if self.flag_porcelain || self.flag_z { Format::Porcelain }
        else { Format::Long }
    }
}

fn main() {
    const USAGE: &'static str = "
usage: status [options] [--] [<spec>..]

Options:
    -s, --short                 show short statuses
    --long                      show longer statuses (default)
    --porcelain                 ??
    -b, --branch                show branch information
    -z                          ??
    --ignored                   show ignored files as well
    --untracked-files <opt>     setting for showing untracked files [no|normal|all]
    --ignore-submodules <opt>   setting for ignoring submodules [all]
    --git-dir <dir>             git directory to analyze
    --repeat                    repeatedly show status, sleeping inbetween
    --list-submodules           show submodules
    -h, --help                  show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
