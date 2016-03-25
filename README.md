# churn

Count how many versions exist of each file in a git repository.

Usage:

    $ churn
          1 .gitignore
          4 Cargo.lock
          4 Cargo.toml
          1 README.md
         13 src/main.rs

This prints the name of every file that ever existed in `HEAD` or any
preceding commit, along with the number of different versions of that
file in those commits.

Build it with `--release`: some Git repositories are pretty big!


## How to install

* Install [Rust](https://www.rust-lang.org/).
* Clone this repository and `cd` into it.
* `cargo build --release`
* This builds the `churn` executable under `./target/release`. Copy it to a directory in your PATH.
