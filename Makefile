run:
	SKIP_WASM_BUILD= cargo run -- --dev --execution native

toolchain:
	./scripts/init.sh

build-wasm:
	WASM_BUILD_TYPE=release cargo build

init: toolchain build-wasm

check:
	SKIP_WASM_BUILD= cargo check

build:
	SKIP_WASM_BUILD= cargo build

purge:
	SKIP_WASM_BUILD= cargo run -- purge-chain --dev -y

restart: purge run
