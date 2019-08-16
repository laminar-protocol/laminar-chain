#!/usr/bin/env bash

set -e

cargo run --release -- purge-chain --dev -y
cargo run --release -- --dev
