#!/usr/bin/env bash
# Builds the snap from the latest CI .deb and installs it locally for testing.
# Run after the CI run for your last push has finished.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION=$(grep '^version:' snap/snapcraft.yaml | cut -d'"' -f2)
echo "Waiting for CI…"
gh run watch --repo quietbytehq/stingray --exit-status || true
rm -rf /tmp/stingray-build
gh run download --repo quietbytehq/stingray -n stingray-linux-x86_64 -D /tmp/stingray-build
mkdir -p dist && rm -f dist/*.deb
cp "/tmp/stingray-build/deb/stingray_${VERSION}_amd64.deb" dist/
snapcraft pack
sudo snap install --dangerous "./stingray_${VERSION}_amd64.snap"
snap list stingray
echo "Done. Start it with: snap run stingray"
