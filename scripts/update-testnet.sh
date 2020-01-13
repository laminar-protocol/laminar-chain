#!/usr/bin/env bash

set -e

cargo clean
WASM_BUILD_TYPE=release cargo run -- build-spec --chain testnet-latest > ./resources/testnet.json
WASM_BUILD_TYPE=release cargo run -- build-spec --chain ./resources/testnet.json --raw > ./resources/testnet-dist.json
