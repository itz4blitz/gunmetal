# Gunmetal

An open-source media server and player for music, movies and TV, built so the
server almost never has to transcode: the client plays the original file, and
the server mostly just sends bytes.

**Status: pre-alpha.** Nothing here is usable yet. The only code that exists
is the first piece of the Matroska parser. See
[the architecture record](docs/adr/0001-architecture.md) for where this is
going and why.

## What it will be

| Part | Built in | Job |
|---|---|---|
| Core | Rust crate | Container parsers, remuxer, playback decisions, protocol types. Shared by everything below. |
| Server | Rust, single binary, SQLite | Indexes the library, serves original bytes, remuxes in-process, hands real transcodes to a sandboxed FFmpeg. |
| Client | React Native, TypeScript | One UI for TVs, browsers and desktop, with a local copy of the library. |
| Mobile app | React Native, TypeScript | Phones and tablets, including offline downloads. |
| Site | Cloudflare | Docs and landing page at [gunmetal.tv](https://gunmetal.tv). |

## Repository layout

```
crates/gunmetal-core   shared core (the only crate so far)
docs/adr               architecture decision records
scripts/gate.sh        every quality gate, in one script
```

The server, client, mobile app and site get their directories when their
first test is written.

## Development

You need Rust 1.85 or newer, plus
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) and
[cargo-mutants](https://mutants.rs).

```bash
scripts/gate.sh
```

That runs the formatter, the linter, the tests with a 100% coverage
requirement, and mutation testing with a zero-survivor requirement. CI runs
the same script. See [CONTRIBUTING.md](CONTRIBUTING.md) for the testing rules.

## License

[AGPL-3.0-or-later](LICENSE).
