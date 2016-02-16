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
use git2::{Repository, Error, Revwalk, Oid, Tree, Object, ObjectType};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;

#[derive(RustcDecodable)]
struct Args {
    flag_topo_order: bool,
    flag_date_order: bool,
    flag_reverse: bool,
}

/*
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
*/

type ChurnData = BTreeMap<String, HashSet<Oid>>;

fn join(base: &str, name: &str) -> String {
    match base {
        "" => name.to_string(),
        _ => base.to_string() + "/" + name
    }
}

fn update_churn_data_for_object(repo: &Repository, path: String, obj: &Object, results: &mut ChurnData) -> Result<(), Error> {
    let added_entry = {
        let result_entry = results.entry(path.clone());
        let hashes = result_entry.or_insert_with(HashSet::new);
        hashes.insert(obj.id())
    };
    if added_entry {
        match obj.kind() {
            Some(ObjectType::Tree) => {
                let subtree = obj.as_tree().unwrap();
                try!(update_churn_data_for_tree(repo, &path, subtree, results));
            }
            _ => {}
        }
    }
    Ok(())
}

fn update_churn_data_for_tree(repo: &Repository, path: &str, tree: &Tree, results: &mut ChurnData) -> Result<(), Error> {
    for entry in tree.iter() {
        let child_object = try!(entry.to_object(repo));
        let child_path = join(path, entry.name().unwrap());
        try!(update_churn_data_for_object(repo, child_path, &child_object, results));
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

const COMMITS_PER_DOT: usize = 1000;

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open("."));
    let mut revwalk = try!(repo.revwalk());
    let spec = "HEAD";

    config_revwalk(&mut revwalk, args);

    let mut churn_data: ChurnData = ChurnData::new();

    let id:Oid = try!(repo.revparse_single(spec)).id();
    try!(revwalk.push(id));
    let mut n = 0;
    for id in revwalk {
        let commit = try!(repo.find_commit(id));
        let tree = try!(commit.tree());
        try!(update_churn_data_for_object(&repo, "".to_string(), tree.as_object(), &mut churn_data));
        //let my_commit = git_commit_to_my_commit(commit);
        //println!("{} {}", my_commit.oid, my_commit.summary);

        n += 1;
        if n % COMMITS_PER_DOT == 0 {
            print!(".");
        }
        std::io::stdout().flush().unwrap();
    }
    println!("");

    for (filename, hashes) in churn_data {
        println!("{}, {}", filename, hashes.len());
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
