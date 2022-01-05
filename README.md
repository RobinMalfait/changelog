# Changelog

A simple tool to make changes to your `CHANGELOG.md` file easier, if they
follow the [keep a changelog](https://keepachangelog.com/en/1.0.0/) convention.

## Requirements

- This is a Rust project and the binaries are not published
  anywhere. This means that you need to have Rust/Cargo installed.
- This tool talks to the GitHub API, therefore you need to have a
  `GITHUB_API_TOKEN` environment variable.

## Installation

```sh
cargo build --release
```

This will create a `changelog` binary at `./target/release/changelog`.

### Optional quality of life improvements

I am using `zshrc`, and I make sure to export the `./target/release/` folder so
that I can automatically run the `changelog` binary.

```sh
PATH=./target/release:$PATH
```

In addition, I also have this `PATH`, so that I can run `changelog` from
anywhere on my system.

```sh
PATH=/path-to-changelog-project/target/release:$PATH
```

## API

Every command has the following options:

```
-f, --filename <FILENAME>    The changelog filename [default: CHANGELOG.md]
-h, --help                   Print help information
    --pwd <PWD>              The current working directory [default: .]
```

### Adding new entries to the `CHANGELOG.md` file

Every command behaves exactly the same and will add a new entry to the
`CHANGELOG.md` file in their own section.

You can use a GitHub link to a PR, issue, commit or discussion. This will add a
link with the title of the resource from above and a link to it.

```sh
changelog <command> https://github.com/<owner>/<repo>/pull/<number>
```

If you want to write your own message instead of fetching the title from the
GitHub resource, then you can use the `-m` or `--message` flag instead:

```sh
changelog <command> -m "My new changelog entry"
```

Here is a list of all the commands and their sections:

- `changelog add` adds a new entry to the `### Added` section
- `changelog fix` adds a new entry to the `### Fixed` section
- `changelog change` adds a new entry to the `### Changed` section
- `changelog remove` adds a new entry to the `### Removed` section
- `changelog deprecate` adds a new entry to the `### Deprecated` section

### `changelog notes`

This will print out the contents of a version as plain text, which is useful if
you want to get this data to insert into your release notes.

By default we will print out the notes from the `[Unreleased]` section if
entries already exist in that section. If not, then we will copy the contents
of the `latest` version.

- `changelog notes unreleased`, this will _always_ print the notes of the
  `[Unreleased]` section, even if nothing exists yet.
- `changelog notes latest`, this will _always_ print the notes of the newest
  version in the list.
- `changelog notes 3.0.5`, this will print the notes of a specific version.

### `changelog list`

This will allow you to list the available versions (without the notes) as a
quick summary. By default we will list the 10 most recent versions.

- `-a, --amount <AMOUNT>` Amount of versions to show [default: 10]
- `--all` Shorthand for "--amount all"

E.g.:

```shellsession
$ changelog list
- unreleased      https://github.com/<owner>/<repo>/compare/v0.1.0...HEAD
- 0.1.0           https://github.com/<owner>/<repo>/releases/tag/v0.1.0
```

### `changelog release`

TODO
