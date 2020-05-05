use crate::chain_spec;
use crate::cli::{Cli, Subcommand};
use crate::executor::Executor;
use crate::service;
use runtime::{Block, RuntimeApi};
use sc_cli::SubstrateCli;

impl SubstrateCli for Cli {
	fn impl_name() -> &'static str {
		"LaminarChain"
	}

	fn impl_version() -> &'static str {
		env!("SUBSTRATE_CLI_IMPL_VERSION")
	}

	fn description() -> &'static str {
		"laminar-chain"
	}

	fn author() -> &'static str {
		"Laminar Developers"
	}

	fn support_url() -> &'static str {
		"https://github.com/laminar-protocol/laminar-chain/issues"
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn executable_name() -> &'static str {
		env!("CARGO_PKG_NAME")
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_config()),
			"local" => Box::new(chain_spec::local_testnet_config()),
			"" | "turbulence" => Box::new(chain_spec::laminar_turbulence_config()?),
			"turbulence-latest" => Box::new(chain_spec::laminar_turbulence_latest_config()),
			path => Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		})
	}
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node(service::new_light, service::new_full, runtime::VERSION)
		}

		Some(Subcommand::Base(subcommand)) => {
			let runner = cli.create_runner(subcommand)?;
			runner.run_subcommand(subcommand, |config| Ok(new_full_start!(config).0))
		}

		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run::<Block, RuntimeApi, Executor>(config))
		}

		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;

				runner.sync_run(|config| cmd.run::<Block, Executor>(config))
			} else {
				println!(
					"Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
				);
				Ok(())
			}
		}
	}
}
