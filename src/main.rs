mod chain_spec;
mod rpc;
#[macro_use]
mod service;
mod cli;
mod command;
mod executor;

fn main() -> sc_cli::Result<()> {
	let version = sc_cli::VersionInfo {
		name: "LaminarChain",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "laminar",
		author: "Laminar Developers",
		description: "laminar-chain",
		support_url: "https://github.com/laminar-protocol/laminar-chain/issues",
		copyright_start_year: 2019,
	};

	command::run(version)
}
