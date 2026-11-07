# Changelog

All notable changes to DigUp are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- README rewrite with install, optional components and build instructions.
- `LICENSE` (MIT), security policy, contributing guide, code of conduct, issue and PR templates.
- `ci.yml` (format, tests, clippy on pull requests and pushes), a tag-driven `release.yml` that
  publishes the validated zip with `SHA256SUMS.txt`, and Dependabot for Cargo and GitHub Actions.
- Acknowledgement of the upstream ARahim3/DigUp project that inspired this one.
- Package metadata in `Cargo.toml` (license, repository, readme, keywords).

### Changed

- `package.ps1` derives the default archive name from the `Cargo.toml` version instead of a
  hard-coded one.
- The validation workflow can be reused by the release workflow (`workflow_call`).

## [0.1.2]

### Added

- Windows Rust desktop search: exact, regex, wildcard and folder search; TXT, Markdown, RST, PDF,
  DOCX and ODT extraction; optional OCR and multimedia indexing; native folder watching and
  previews; optional local semantic search with EmbeddingGemma 2 prompts; per-user install and
  uninstall scripts; headless release validation on Windows runners.

[Unreleased]: https://github.com/boss477/digup-windows/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/boss477/digup-windows/releases/tag/v0.1.2
