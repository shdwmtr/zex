#!/bin/sh
# Invoked by semantic-release's @semantic-release/exec "prepareCmd" once it
# has decided a release is warranted, with the version it computed. Stamps
# that version into Cargo.toml/Cargo.lock so the crate itself matches the
# git tag and GitHub Release semantic-release is about to create.
set -eu

VERSION="$1"

sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm -f Cargo.toml.bak

# Refreshes Cargo.lock's own [[package]] entry for zex to match, without
# touching any dependency versions (no network access needed).
cargo check --profile dev --quiet
