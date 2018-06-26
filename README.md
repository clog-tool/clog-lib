clog
====

[![Join the chat at https://gitter.im/thoughtram/clog](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/thoughtram/clog?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

[![Build Status](https://travis-ci.org/clog-tool/clog-lib.png?branch=master)](https://travis-ci.org/thoughtram/clog)

A library for generating a [conventional][convention] changelog from git metadata, written in Rust

[convention]: https://github.com/ajoslin/conventional-changelog/blob/a5505865ff3dd710cf757f50530e73ef0ca641da/conventions/angular.md

## About

`clog` creates a changelog automatically from your local git metadata. See the `clog`s [changelog.md](https://github.com/clog-tool/clog-lib/blob/master/changelog.md) for an example.

The way this works, is every time you make a commit, you ensure your commit subject line follows the [conventional](https://github.com/ajoslin/conventional-changelog/blob/a5505865ff3dd710cf757f50530e73ef0ca641da/conventions/angular.md) format.

*NOTE:* `clog` also supports empty components by making commit messages such as `alias: message` or `alias(): message` (i.e. without the component)


## Usage

There are two ways to use `clog`, as a binary via the command line (See [clog-cli](https://github.com/clog-tool/clog-cli) for details) or as a library in your applicaitons.

See the [documentation](http://clog-tool.github.io/clog-lib/) for information on using `clog` in your applications.

In order to see it in action, you'll need a repository that already has some of those specially crafted commit messages in it's history. For this, we'll use the `clog` repository itself.

 1. Clone the `clog-lib` repository (we will clone to our home directory to make things simple, feel free to change it)

```sh
$ git clone https://github.com/thoughtram/clog ~/clog
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
    let mut clog = Clog::with_dir("~/clog").unwrap_or_else(|e| {
        // Prints the error message and exits
        e.exit();
    });

    // Set some options
    clog.repository("https://github.com/thoughtram/clog")
        .subtitle("Crazy Dog")
        .changelog("changelog.md")
        .from("6d8183f")
        .version("0.1.0");

    // Write the changelog to the current working directory
    //
    // Alternatively we could have used .write_changelog_to("/somedir/some_file.md")
    clog.write_changelog().unwrap_or_else(|e| {
        e.exit();
    });
}
```

 4. Compile and run `$ cargo build --release && ./target/release/bin_name
 5. View the output in your favorite markdown viewer! `$ vim changelog.md`

### Default Options

`clog` can also be configured using a default configuration file so that you don't have to specify all the options each time you want to update your changelog. To do this add a `.clog.toml` file to your repository.

```toml
[clog]
# A repository link with the trailing '.git' which will be used to generate
# all commit and issue links
repository = "https://github.com/thoughtram/clog"
# A constant release title
subtitle = "my awesome title"

# specify the style of commit links to generate, defaults to "github" if omitted
link-style = "github"

# The preferred way to set a constant changelog. This file will be read for old changelog
# data, then prepended to for new changelog data. It's the equivilant to setting
# both infile and outfile to the same file.
#
# Do not use with outfile or infile fields!
#
# Defaults to stdout when omitted
changelog = "mychangelog.md"

# This sets an output file only! If it exists already, new changelog data will be
# prepended, if not it will be created.
#
# This is useful in conjunction with the infile field if you have a separate file
# that you would like to append after newly created clog data
#
# Defaults to stdout when omitted
outfile = "MyChangelog.md"

# This sets the input file old! Any data inside this file will be appended to any
# new data that clog picks up
#
# This is useful in conjunction with the outfile field where you may wish to read
# from one file and append that data to the clog output in another
infile = "My_old_changelog.md"

# This sets the output format. There are two options "json" or "markdown" and
# defaults to "markdown" when omitted
output-format = "json"

# If you use tags, you can set the following if you wish to only pick
# up changes since your latest tag
from-latest-tag = true
```

### Custom Sections

By default, `clog` will display three sections in your changelog, `Features`, `Performance`, and `Bug Fixes`. You can add additional sections by using a `.clog.toml` file. To add more sections, simply add a `[sections]` table, along with the section name and aliases you'd like to use in your commit messages:

```toml
[sections]
MySection = ["mysec", "ms"]
```

Now if you make a commit message such as `mysec(Component): some message` or `ms(Component): some message` there will be a new "MySection" section along side the "Features" and "Bug Fixes" areas.

*NOTE:* Sections with spaces are suppported, such as `"My Special Section" = ["ms", "mysec"]`

### Component Aliases

By default, `clog` will use the exact component string given in your
commit message (i.e. `feat(comp): message` will be displayed as as the
"comp" component in the changelog output.  If you want to display a
longer string for a component in your changelog, you can define aliases
in a `[components]` table in your `.clog.toml` configuration file:

```toml
[components]
MyLongComponentName = ["long", "comp"]
```

With this configuration, `feat(comp): message` will be displayed as the
"MyLongComponentName" component in the changelog output.

## Companion Projects

- [Commitizen](http://commitizen.github.io/cz-cli/) - A command line tool that helps you writing better commit messages.

## LICENSE

clog is licensed under the MIT Open Source license. For more information, see the LICENSE file in this repository.
