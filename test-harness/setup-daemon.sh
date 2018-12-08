#!/usr/bin/env sh
mkdir -p test_data || true
git daemon --base-path=. --export-all --enable=receive-pack --reuseaddr --informative-errors --verbose
