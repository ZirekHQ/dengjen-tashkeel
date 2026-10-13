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
type — `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `style:`, `ci:`, `build:` — lets
the release tooling compute the next version from history (see **Releasing**) and keeps `git log`
skimmable. The repository keeps no `CHANGELOG` file; releases are published on
[GitHub Releases](https://github.com/ZirekHQ/dengjen-tashkeel/releases). Mark a breaking change
with `feat!:` or a `BREAKING CHANGE:` footer so it triggers a major bump. Example:
`fix: point repository metadata at the ZirekHQ fork`. See [conventionalcommits.org](https://www.conventionalcommits.org/) for the
full spec.

Not following either convention won't block your PR — a maintainer may just tweak the title or
ask you to sign before merging.

## Releasing

Maintainers only. `Cargo.toml`'s `[workspace.package].version` is the single
source of truth every published artifact tracks in lockstep -- there's no
reason for the Rust crates, Python wheels, Java jars, or the vcpkg/Conan
packages to diverge, so don't let them.

1. Run the **Prepare release** workflow (`workflow_dispatch`, from the
   Actions tab). Only an `admin` or `maintain` collaborator can, and only
   from `main`. It computes the next semver version from Conventional
   Commit subjects merged since the last `vX.Y.Z` tag (`fix:`/etc -> patch,
   `feat:` -> minor, `feat!:`/`BREAKING CHANGE:` -> major;
   docs/chore/style/refactor/test-only since the last tag means no release).
   `dry_run` defaults to true: the plan lands in the run summary and
   nothing else happens. Untick it to open a PR bumping every hand-synced
   copy of the version (`scripts/next-version.sh` /
   `scripts/bump-version.sh`).
2. Review and merge that PR once its required checks pass. **The PR's
   checks are the release gate** -- merging it releases the version in the
   diff, with nothing further to confirm or wait for:
   [`release.yml`](workflows/release.yml) tags that merge
   commit `vX.Y.Z` and triggers `github-release.yml` (GitHub Release),
   `publish-python.yml` (PyPI), `publish-java.yml` (Maven Central), and
   `publish-crates.yml` (crates.io) automatically. The GitHub Release body opens with
   a "What's changed" section that `scripts/release-notes.sh` builds from the Conventional
   Commit subjects since the previous tag (dependency updates are counted, not listed), so a
   clear PR title is the changelog entry. If any of them fails
   partway through, use GitHub's "Re-run failed jobs" on that `release.yml`
   run (Actions tab): it re-runs just the failed job(s) against the run's
   own commit, and every publish step is idempotent (skips a
   crate/package/artifact already published), so no new tag is needed.
3. The bump PR is opened with the `zirekhq-automation` GitHub App's token so
   its checks run. It needs `contents: write` and `pull-requests: write`, is
   installed on this repo, and its org secrets `AUTOMATION_APP_CLIENT_ID` and
   `AUTOMATION_APP_PRIVATE_KEY` must be shared with this repo. Without
   them the PR opens with `GITHUB_TOKEN` and its checks don't start until
   someone closes and reopens it.
   `publish-python.yml`'s `environment: pypi` and `publish-java.yml`'s
   `environment: maven-central` (which holds the `JRELEASER_*` secrets)
   stay as the per-registry approval points.
4. Once the release archives exist, refresh the vcpkg port and Conan
   recipe's checksums against them and open a PR -- see
   [packaging/README.md](../packaging/README.md). These necessarily lag one
   PR behind the tag (real per-platform hashes can't exist before the
   archives do), but should always land on the *same* version number as the
   release they point to, never their own.

## Getting started

See [README.md](../README.md) for build instructions.

## Development checks

CI (`CI.yml`, which calls `rust-lint.yml`, `java-test.yml` and `python-wheel.yml`, and ends in the required `build` check) enforces these; run them before opening a PR:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --no-fail-fast`
- `cargo clippy -p dengjen-tashkeel --no-default-features --features ort-dylib -- -D warnings`,
  and the same for `-p dengjen-tashkeel-python` with `--all-targets` (the `ort-dylib` path the
  wheel ships)
  and `cargo clippy -p dengjen-tashkeel-capi --no-default-features --features ort-dylib -- -D warnings`
  (the build the Java jars ship)
- `./gradlew check` in `bindings/java`, with `ORT_DYLIB_PATH` pointing at an `onnxruntime` shared
  library (the Java natives load it dynamically)
- `modelVersion` in `bindings/java/build.gradle.kts` must be bumped whenever
  `crates/core/data/ort/model.onnx` changes; the release publishes the
  `dengjen-tashkeel-model` artifact only for a version not yet on Maven Central
- `cargo check --workspace --locked` on Rust 1.88.0 (the minimum supported version)
- `cargo deny check licenses bans sources`, `cargo audit`
- `cargo llvm-cov --workspace --fail-under-lines 80` (CI's `lint / coverage` job; workspace line
  coverage must stay at or above 80%)

## Pull requests from forks

Forks never receive `SONAR_TOKEN`. `sonar-fork-coverage.yml` builds the
coverage report from the fork's head with no secrets, and
`sonar-fork-scan.yml` then scans that artifact with the token and posts the
real `SonarCloud Code Analysis` status to the head commit. The required check
stays pending until the scan finishes, so a fork PR cannot merge without a
quality-gate result. A maintainer can comment `/sonar` on the PR to re-run the
scan.
