`clog`

![Rust Version][rustc-image]
[![crates.io][crate-image]][crate-link]
[![Dependency Status][deps-image]][deps-link]
[![docs-image][docs-image]][docs-link]

A library for generating a [conventional][convention] changelog from git
metadata, written in Rust

## About

`clog` creates a changelog automatically from your local git metadata. See the
`clog`s [changelog.md][our_changelog] for an example.

The way this works, is every time you make a commit, you ensure your commit
subject line follows the [conventional][convention] format.

*NOTE:* `clog` also supports empty components by making commit messages such as
`alias: message` or `alias(): message` (i.e. without the component)

## Usage

There are two ways to use `clog`, as a binary via the command line (See
[clog-cli][clog_cli] for details) or as a library in your applications.

See the [documentation][docs-link] for information on using `clog` in your
applications.

In order to see it in action, you'll need a repository that already has some of
those specially crafted commit messages in it's history. For this, we'll use
the `clog` repository itself.

 1. Clone the `clog-lib` repository (we will clone to our home directory to
    make things simple, feel free to change it)

```sh
$ git clone https://github.com/clog-tool/clog-lib
```

 2. Add `clog` as a dependency in your `Cargo.toml`

```toml
[dependencies]
clog = "*"
```

 3. Use the following in your `src/main.rs`

```rust
extern crate clog;

use clog::Clog;

fn main() {
    // Create the struct
    let mut clog = Clog::with_git_work_tree("~/clog")
        .unwrap()
        .repository("https://github.com/thoughtram/clog")
        .subtitle("Crazy Dog")
        .changelog("changelog.md")
        .from("6d8183f")
        .version("0.1.0");

    // Write the changelog to the current working directory
    //
    // Alternatively we could have used .write_changelog_to("/somedir/some_file.md")
    clog.write_changelog().unwrap();
}
```

 4. Compile and run `$ cargo build --release && ./target/release/bin_name
 5. View the output in your favorite markdown viewer! `$ vim changelog.md`

### Configuration

`clog` can also be configured using a configuration file in TOML.

See the `examples/clog.toml` for available options.

## Companion Projects

- [`clog-cli`](http://github.com/clog-tool/clog-cli/) - A command line tool
  that uses this library to generate changelogs.
- [Commitizen](http://commitizen.github.io/cz-cli/) - A command line tool that
  helps you writing better commit messages.

## LICENSE

clog is licensed under the MIT Open Source license. For more information, see the LICENSE file in this repository.

[//]: # (badges)

[docs-image]: https://img.shields.io/docsrs/clog
[docs-link]: https://docs.rs/clog
[rustc-image]: https://img.shields.io/badge/rustc-1.56+-blue.svg
[crate-image]: https://img.shields.io/crates/v/clog.svg
[crate-link]: https://crates.io/crates/clog
[deps-image]: https://deps.rs/repo/github/clog-tool/clog-lib/status.svg
[deps-link]: https://deps.rs/repo/github/clog-tool/clog-lib/


[//]: # (Links)

[convention]: https://github.com/ajoslin/conventional-changelog/blob/a5505865ff3dd710cf757f50530e73ef0ca641da/conventions/angular.md
[our_changelog]: https://github.com/clog-tool/clog-lib/blob/master/changelog.md
[clog_cli]: https://github.com/clog-tool/clog-cli
