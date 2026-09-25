# Spacers: an example printCAD workbench

A workbench package for [printCAD](https://github.com/gilbertorconde/printCAD)
that makes round, hex, square and flanged spacers, and a starting point
for your own. It
shows the parts most workbenches need:

- a tool per shape (Round `R`, Hex `H`, Square `S`, Flanged `F`) that makes
  a feature on a body of its own and opens its task panel;
- a feature kind whose numbers take formulas anywhere printCAD shows them;
- a rebuild plan the kernel runs to make the solid (a flanged spacer is two
  extrusions fused);
- data that reads across versions: spacers saved with 0.1.0 open in 0.2.0,
  whose flange fields default to none;
- the spacer's axis and size drawn in the view while it is edited;
- a command for scripts and AI agents: `pc.example.spacers.make{outer = 8, height = 10}`;
- an "Edit spacer" entry on the tree's menu, and a settings page for the
  sizes new spacers start from.

## Install it

In printCAD, Preferences › Workbench packages, type this repository's
address, `https://github.com/gilbertorconde/PrintCAD-example-wb`, and press
Install. printCAD takes the latest release's `.pcbench` file, checks for
newer releases, and offers to update. The workbench loads the next time
printCAD starts.

## Build it

With [rustup](https://rustup.rs), the toolchain and the `wasm32-wasip2`
target come from `rust-toolchain.toml`:

```sh
scripts/pack.sh
```

builds the component and writes `dist/example.spacers-<version>.pcbench`,
which Preferences › Workbench packages › Install from a file… takes.

```sh
cargo test --target x86_64-unknown-linux-gnu   # the package's own logic
```

## Release it

1. Set the same version in `bench.toml` and `Cargo.toml`.
2. Commit, tag and push: `git tag v0.2.0 && git push origin v0.2.0`.

The Release workflow builds the package, checks that the tag names that
version, and publishes a GitHub release with the `.pcbench` attached. A
tag with a suffix (`v0.3.0-beta.1`) becomes a prerelease, which printCAD
does not offer as an update. Every push and pull request runs the CI
workflow: format, clippy, tests, and the package as a build artifact.

## Make your own

1. Change `id`, `name` and `feature_kinds` in `bench.toml`. The id is
   lowercase and reverse-domain (`io.github.you.thing`); feature kinds, tool,
   action and command ids all start with it.
2. Rename the crate in `Cargo.toml`.
3. Replace `src/lib.rs`; the `Bench` trait's methods all have defaults.
4. Icons are 24×24 SVGs in `icons/`, drawn in white with a 1.5 px stroke;
   name them by file name.

The SDK comes from the printCAD repository (`sdk/printcad-bench-sdk`). Its
guide is [docs/PLUGINS.md](https://github.com/gilbertorconde/printCAD/blob/master/docs/PLUGINS.md).

## Layout

| Path | What it is |
| --- | --- |
| `bench.toml` | the manifest: id, version, feature kinds, capabilities |
| `src/lib.rs` | the workbench |
| `icons/` | its icons |
| `scripts/pack.sh` | build and pack a `.pcbench` |
| `.github/workflows/ci.yml` | checks on every push |
| `.github/workflows/release.yml` | a release for every `v*` tag |

## License

MIT or Apache-2.0, at your option.
