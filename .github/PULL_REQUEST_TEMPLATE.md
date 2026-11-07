## What does this change?

<!-- One or two sentences. Link the issue: "Fixes #123". -->

## How was it tested?

- [ ] `./build.ps1 -TestsOnly` passes (fmt, tests, clippy)
- [ ] Added or updated a test for this change
- [ ] Tried it in the app (`./run.ps1`)
- [ ] Packaging/installer changes still pass the `verify-*.ps1` scripts

## Checklist

- [ ] `CHANGELOG.md` updated (Unreleased)
- [ ] README updated if user-facing
- [ ] No new network calls or telemetry; helper programs get paths as arguments, not shell strings
