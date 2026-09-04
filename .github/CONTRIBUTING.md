# Contributing to Dengjen

Thanks for taking the time to contribute. Two lightweight conventions make reviews easier and
keep the project's history useful — neither is required to get a PR merged.

## Commit signing (recommended)

A signed commit lets anyone verify it actually came from you, not someone spoofing your name and
email. GitHub marks signed commits "Verified," and it's one of the cheapest supply-chain
protections available. See [GitHub's guide to commit signing](https://docs.github.com/en/authentication/managing-commit-signature-verification)
for GPG, SSH, or S/MIME setup — a few minutes, one time.

## Conventional Commit PR titles (recommended)

We squash-merge, so the PR title becomes the commit that lands on `main`. Prefixing it with a
type — `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:` — lets us auto-generate changelogs
and keeps `git log` skimmable. Example: `fix: point repository/documentation metadata at ZirekHQ
fork, not upstream`. See [conventionalcommits.org](https://www.conventionalcommits.org/) for the
full spec.

Not following either convention won't block your PR — a maintainer may just tweak the title or
ask you to sign before merging.

## Releasing

Maintainers only. `Cargo.toml`'s `[workspace.package].version` is the single
source of truth every published artifact tracks in lockstep -- there's no
reason for the Rust crates, Python wheels, Java jars, or the vcpkg/Conan
packages to diverge, so don't let them.

1. Run the **Prepare release** workflow (`workflow_dispatch`, from the
   Actions tab). It computes the next semver version from Conventional
   Commit subjects merged since the last `vX.Y.Z` tag (`fix:`/etc -> patch,
   `feat:` -> minor, `feat!:`/`BREAKING CHANGE:` -> major; docs/chore/style
   /refactor/test-only since the last tag means no release) and opens a PR
   bumping every hand-synced copy of it (`scripts/next-version.sh` /
   `scripts/bump-version.sh`).
2. Review and merge that PR. **This is the release gate** -- merging it
   releases the version in the diff, with nothing further to confirm:
   [`tag-and-release.yml`](workflows/tag-and-release.yml) tags that merge
   commit `vX.Y.Z` and triggers `release.yml` (GitHub Release),
   `python-publish.yml` (PyPI), `java-publish.yml` (Maven Central), and
   `publish.yml` (crates.io) automatically. If any of them fails partway
   through, re-run that specific workflow (Actions tab, or `gh workflow
   run`) rather than pushing a new tag -- `publish.yml`'s steps are
   idempotent, and the others already support re-running against an
   existing tag via their own `workflow_dispatch` input.
3. Once the release archives exist, refresh the vcpkg port and Conan
   recipe's checksums against them and open a PR -- see
   [packaging/README.md](../packaging/README.md). These necessarily lag one
   PR behind the tag (real per-platform hashes can't exist before the
   archives do), but should always land on the *same* version number as the
   release they point to, never their own.

## Getting started

See [README.md](../README.md) for build instructions.
