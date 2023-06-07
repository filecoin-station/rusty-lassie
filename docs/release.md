# Release process

This project follows [semantic versioning](https://semver.org/). The following
documentation will refer to `X.Y.Z` as _major_, _minor_ and _patch_ version.

## Releasing one or more crates

### Prerequisites

- [cargo release](https://github.com/crate-ci/cargo-release/)

### Steps

1. Make sure you have the latest version of the `main` branch:

   ```sh
   $ git checkout main && git pull
   ```

1. Create the new release, replace `0.1.2` with the NEW version n

   ```sh
   $ cargo release --sign-tag --execute 0.1.2
   ```

   Instead of a specific version number, you can also use one of the semver
   types: `patch`, `minor` or `major`.

1. Open the GitHub Releases page:
   https://github.com/filecoin-station/rusty-lassie/releases and draft a new
   release. Use the git tag created in the previous step.

1. Click on the button `Generate release notes`. Review the list of commits and
   pick a few notable ones. Add a new section `## Highlights ✨` at the top of
   the release notes and describe the selected changes.

1. Click on the green button `Publish release`
