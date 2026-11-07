# Contributing to DigUp

Thanks for helping. DigUp is a privacy-conscious Windows search tool: exact search must always work
offline, and anything that sends content elsewhere must be explicit and user-configured.

## Ground rules

- Be kind. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).
- Open an issue before a large change so we can agree on the approach.
- Keep pull requests focused: one change, one reason.
- Never add telemetry or silent network calls.

## Setup

Requires Windows 10/11 x64, [Rust](https://rustup.rs) stable, Visual Studio C++ Build Tools and the
Windows SDK.

```powershell
git clone https://github.com/boss477/digup-windows.git
cd digup-windows
./build.ps1 -TestsOnly   # cargo fmt --check, cargo test --locked, cargo clippy -D warnings
./run.ps1
```

## Before you open a pull request

- `./build.ps1 -TestsOnly` passes. Format with `cargo fmt`.
- Add or update a test for behavior changes (see the `#[cfg(test)]` modules in `src/`).
- Changes to packaging or the installer should still pass `verify-package.ps1`,
  `verify-install.ps1` and `verify-clean-install.ps1` (CI runs them).
- Add a line under **Unreleased** in [CHANGELOG.md](CHANGELOG.md).
- Update the README if the change is user-facing.

## Code guidelines

- Pass file paths to helper programs as arguments, never through a shell string.
- Search and extraction must respect the cancel flag so the UI stays responsive.
- New dependencies need a reason and a license compatible with the notice collection in
  `collect-notices.ps1`.

## Commit messages

Short, imperative, with a type prefix: `feat:`, `fix:`, `docs:`, `test:`, `chore:`, `ci:`,
`refactor:`.

## Releases

Maintainers bump `version` in `Cargo.toml`, update the changelog, and push a `v*` tag. The release
workflow validates and publishes.
