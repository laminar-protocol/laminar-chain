mod chain_spec;
mod cli;
mod service;

pub use sc_cli::{error, IntoExit, VersionInfo};

fn main() {
	let version = VersionInfo {
		name: "Flowchain",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "flowchain",
		author: "Laminar Developers",
		description: "flowchain",
		support_url: "https://github.com/laminar-protocol/flowchain/issues",
	};

	if let Err(e) = cli::run(::std::env::args(), cli::Exit, version) {
		eprintln!("Fatal error: {}\n\n{:?}", e, e);
		std::process::exit(1)
	}
}
