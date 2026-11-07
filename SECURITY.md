# Security Policy

## Supported versions

Security fixes go into the latest release and the `main` branch.

## Reporting a vulnerability

Please **do not open a public issue** for security problems. Report them privately through
[GitHub's private vulnerability reporting](https://github.com/boss477/digup-windows/security/advisories/new).
Include steps to reproduce, the affected version and Windows build, and the impact.

You will get an acknowledgment within a week. Once a fix ships, you will be credited in the
changelog unless you prefer to stay anonymous.

## What DigUp touches

DigUp reads files you point it at, so the main risks are untrusted file content, helper programs,
and where indexed text is sent.

| Area                | Behavior                                                                                                                                                             |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Exact search        | Runs locally. Nothing leaves the machine.                                                                                                                            |
| Semantic search     | Sends query and document text to the embedding endpoint **you configure**. Use `localhost` to stay local. Source files may contain secrets; `.gitignore` is not applied. |
| Helper programs     | OCR, media and PDF rendering call Tesseract, FFmpeg, Poppler and VLC. Paths come from fixed locations or the `DIGUP_*` environment variables; file paths are passed as arguments, not through a shell. |
| Document extraction | PDF, DOCX and ODT are parsed in-process by Rust libraries.                                                                                                           |
| Install             | Per-user, no admin rights. Uninstall removes only installer-owned files and preserves settings, vectors and models.                                                  |
| Packaging           | Release packaging is blocked on unresolved third-party license declarations; CI actions are pinned to commit SHAs.                                                   |

### Known limitations

- Releases are **unsigned previews**. Verify downloads against the published `SHA256SUMS.txt`.
- Helper programs are separate software with their own security updates; DigUp does not bundle
  them.
- A semantic endpoint over plain `http://` to a remote host exposes indexed text on the network.
  Prefer `localhost` or an HTTPS endpoint you trust.
