#!/usr/bin/env bash

set -exo pipefail

if [ test -d _builddir ]; then
    rm -R _builddir
fi

meson setup _builddir
