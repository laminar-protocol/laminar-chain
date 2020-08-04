use crate::executor::Executor;
use crate::service::new_full_params;
use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service,
};
use runtime::{Block, RuntimeApi};
use sc_cli::{ChainSpec, Result, Role, RuntimeVersion, SubstrateCli};
use sc_service::ServiceParams;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"LaminarChain".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"laminar-chain".into()
	}

	fn author() -> String {
		"Laminar Developers".into()
	}

	fn support_url() -> String {
		"https://github.com/laminar-protocol/laminar-chain/issues".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_config()?),
			"local" => Box::new(chain_spec::local_testnet_config()?),
			"" | "turbulence" => Box::new(chain_spec::laminar_turbulence_config()?),
			"turbulence-latest" => Box::new(chain_spec::laminar_turbulence_latest_config()?),
			path => Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		})
	}

	fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&runtime::VERSION
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| match config.role {
				Role::Light => service::new_light(config),
				_ => service::new_full(config),
			})
		}

		Some(Subcommand::Base(subcommand)) => {
			let runner = cli.create_runner(subcommand)?;

			runner.run_subcommand(subcommand, |config| {
				let (
					ServiceParams {
						client,
						backend,
						import_queue,
						task_manager,
						..
					},
					..,
				) = new_full_params(config)?;
				Ok((client, backend, import_queue, task_manager))
			})
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
