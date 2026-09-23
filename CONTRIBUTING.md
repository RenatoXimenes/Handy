# Contributing to Vozel

Vozel is independently maintained at
[RenatoXimenes/Vozel](https://github.com/RenatoXimenes/Vozel). It is a fork of
[cjpais/Handy](https://github.com/cjpais/Handy), whose governance, support
channels and sponsorship do not apply to Vozel.

## Get started

Install [Rust](https://rustup.rs/), [Bun](https://bun.sh/) and the
platform-specific prerequisites in [BUILD.md](BUILD.md). Then:

```bash
git clone https://github.com/RenatoXimenes/Vozel.git vozel
cd vozel
git remote add upstream https://github.com/cjpais/Handy.git
bun install
bun run tauri dev
```

The checkout directory may use `vozel`.

## Report and propose changes

Search [Vozel issues](https://github.com/RenatoXimenes/Vozel/issues) before
opening a bug report. Include the version, operating system, reproduction
steps, expected and observed behavior, and redacted logs. Do not include API
keys, recordings or private transcripts.

Open a pull request against this repository with one focused change, an
explanation of its purpose and the checks you ran. Feature proposals and
questions may use this repository's GitHub Discussions when enabled; no
upstream discussion, Discord, email or sponsorship channel represents Vozel.

## Development expectations

- Keep changes focused and preserve existing user changes.
- Run `bun run format:check`, `bun run lint`, and relevant tests when they
  apply to the change.
- Add or update documentation for user-visible behavior.
- Disclose material AI assistance in the pull request description.

Translation contributions follow [CONTRIBUTING_TRANSLATIONS.md](CONTRIBUTING_TRANSLATIONS.md).

## Branding and data

Use the Vozel name for new user-facing documentation. The application still
contains legacy Handy identifiers for compatibility with existing settings,
data and upstream dependencies. On first launch, Vozel imports existing data
without removing it. Do not present Vozel as affiliated with or endorsed by
the upstream project.

## License

Contributions to Vozel are licensed under the [MIT License](LICENSE). Preserve
upstream copyright notices and dependency licenses. A Portuguese Brazilian
model is subject to OpenMDW-1.1 when that model is distributed; its license must
ship with that distribution.
