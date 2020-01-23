mod chain_spec;
mod cli;
mod rpc;
mod service;

pub use sc_cli::{error, IntoExit, VersionInfo};

fn main() {
	let version = VersionInfo {
		name: "LaminarChain",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "laminar",
		author: "Laminar Developers",
		description: "laminar-chain",
		support_url: "https://github.com/laminar-protocol/laminar-chain/issues",
	};

	if let Err(e) = cli::run(::std::env::args(), cli::Exit, version) {
		eprintln!("Fatal error: {}\n\n{:?}", e, e);
		std::process::exit(1)
	}
}
