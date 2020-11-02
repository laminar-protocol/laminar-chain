#!/usr/bin/env bash

set -e

# cargo clean
WASM_BUILD_TYPE=release cargo run -- build-spec --chain turbulence-latest > ./resources/turbulence.json
WASM_BUILD_TYPE=release cargo run -- build-spec --chain ./resources/turbulence.json --raw > ./resources/turbulence-dist.json
