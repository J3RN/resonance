#!/usr/bin/env bash

set -exo pipefail

meson compile -C _builddir --verbose
RUST_LOG=debug meson devenv -C _builddir ./src/resonance
