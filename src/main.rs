mod chain_spec;
mod rpc;
#[macro_use]
mod service;
mod cli;
mod command;
mod executor;

fn main() -> sc_cli::Result<()> {
	command::run()
}
