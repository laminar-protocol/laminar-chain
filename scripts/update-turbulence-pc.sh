#!/usr/bin/env bash

set -e

# cargo clean
WASM_BUILD_TYPE=release cargo run --manifest-path bin/laminar/Cargo.toml -- build-spec --chain turbulence-latest > ./resources/turbulence-pc.json
WASM_BUILD_TYPE=release cargo run --manifest-path bin/laminar/Cargo.toml -- build-spec --chain ./resources/turbulence-pc.json --raw > ./resources/turbulence-pc-dist.json
