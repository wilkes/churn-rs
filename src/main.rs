/*
 * libgit2 "rev-list" example - shows how to transform a rev-spec into a list
 * of commit ids
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

extern crate git2;
extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;
use git2::{Repository, Error, Revwalk, Oid, Commit, Tree, Time, ObjectType};
use std::collections::HashMap;

#[derive(RustcDecodable)]
struct Args {
    flag_topo_order: bool,
    flag_date_order: bool,
    flag_reverse: bool,
}

struct MyCommit {
    oid: Oid,
    summary: String,
    committer: String,
    time: Time,
}

fn git_commit_to_my_commit(mut gcommit: Commit) -> MyCommit {
    MyCommit {
        oid: gcommit.id(),
        summary: gcommit.summary().unwrap_or("").to_string(),
        time: gcommit.time(),
        committer: gcommit.committer().name().unwrap_or("<Unknown>").to_string(),
    }
}

type ChurnData = HashMap<String, usize>;

fn join(base: &str, name: &str) -> String {
    match base {
        "" => name.to_string(),
        _ => base.to_string() + "/" + name
    }
}

fn update_churn_data_for_tree(repo: &Repository, dir: &str, tree: &Tree, results: &mut ChurnData) -> Result<(), Error> {
    for entry in tree.iter() {
        match entry.kind() {
            Some(ObjectType::Tree) => {
                let entry_object = try!(entry.to_object(repo));
                let subtree = entry_object.as_tree().unwrap();
                let subdir = join(dir, entry.name().unwrap());
                try!(update_churn_data_for_tree(repo, &subdir, subtree, results));
            }
            Some(ObjectType::Blob) => {
                let full_path = join(dir, entry.name().unwrap());
                let hash_entry = results.entry(full_path);
                let value_ref = hash_entry.or_insert(0);
                *value_ref += 1;
            }
            _ => {}
        }
    }
    Ok(())
}

fn config_revwalk(revwalk: &mut Revwalk, args: &Args) -> () {
    let base = if args.flag_reverse {git2::SORT_REVERSE} else {git2::SORT_NONE};
    let sort_type =  if args.flag_topo_order {
                         git2::SORT_TOPOLOGICAL
                     } else if args.flag_date_order {
                         git2::SORT_TIME
                     } else {
                         git2::SORT_NONE
                     };
    revwalk.set_sorting(base | sort_type);
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open("."));
    let mut revwalk = try!(repo.revwalk());
    let spec = "HEAD";

    config_revwalk(&mut revwalk, args);

    let mut churn_data: ChurnData = HashMap::new();

    let id:Oid = try!(repo.revparse_single(spec)).id();
    try!(revwalk.push(id));
    for id in revwalk {
        let commit = try!(repo.find_commit(id));
        let tree = try!(commit.tree());
        try!(update_churn_data_for_tree(&repo, "", &tree, &mut churn_data));
        //let my_commit = git_commit_to_my_commit(commit);
        //println!("{} {}", my_commit.oid, my_commit.summary);
    }

    for (filename, count) in churn_data {
        println!("{}, {}", filename, count);
    }
    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: rev-list [options]

Options:
    --topo-order        sort commits in topological order
    --date-order        sort commits in date order
    --reverse           sort commits in reverse
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
