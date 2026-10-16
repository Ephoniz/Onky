# Contributing

Audio Program is MIT licensed. Issues and pull requests are welcome. Agents working in this repo follow [AGENTS.md](AGENTS.md).

## Branches and reviews

Work happens on a feature branch. Open a pull request into `main`. Merge with a squash or a rebase so `main` stays linear. Do not merge a branch that is behind `main`.

Use [Conventional Commits](https://www.conventionalcommits.org/) (`feat`, `fix`, `docs`, `test`, `ci`, `chore`). Keep each commit focused.

## Checks

Before asking for review:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo test -p audio-program-engine --release --lib reference_session_cpu_budget -- --nocapture
cargo deny check
```

The CPU budget is the release measurement. A debug run is not that measurement.

## Branch protection

These settings are the recommendation for `main`. They are not applied by this repository's files; a repository admin turns them on in the forge settings.

- Require a pull request before merging into `main`.
- Require at least one approval.
- Require these checks: `fmt`, `clippy`, `test`, `windows`, `deny`.
- Require the branch to be up to date before merging.
- Do not allow force-pushes to `main`.
- Do not allow deletion of `main`.
- Require linear history, matching squash or rebase merges.
- Require conversation resolution before merging.

## Conduct and security

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) and [SECURITY.md](SECURITY.md).
