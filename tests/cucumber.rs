#![cfg(test)]

use cucumber::{after, before, cucumber};

use frame_support::{assert_noop, assert_ok, parameter_types};
use module_primitives::{Balance, Leverage, Leverages, TradingPair};
use orml_utilities::{Fixed128, FixedU128};
use runtime::tests::*;
use runtime::{AccountId, BlockNumber, CurrencyId, LiquidityPoolId};
use sp_runtime::{DispatchResult, PerThing, Permill};

#[derive(Default)]
pub struct World {
	pub ext: Option<sp_io::TestExternalities>,
}

impl World {
	pub fn execute_with<R>(&mut self, execute: impl FnOnce() -> R) -> R {
		self.ext.as_mut().expect("Missing accounts step").execute_with(execute)
	}
}

impl cucumber::World for World {}

fn parse_name(name: Option<&String>) -> AccountId {
	let name = name.expect("Missing name");
	match name.to_ascii_lowercase().trim() {
		"pool" => POOL::get(),
		"alice" => ALICE::get(),
		"bob" => BOB::get(),
		_ => panic!("Invalid account name"),
	}
}

fn parse_dollar(value: Option<&String>) -> Balance {
	let value = value.expect("Missing balance");
	let value = value.replace(" ", "").replace("_", "");
	if value.starts_with("$") {
		let num = value[1..].parse::<f64>().expect("Invalid dollar value");
		((num * (10u64.pow(10) as f64)) as Balance) * 10u128.pow(8) // to avoid accuracy issue when doing conversion
	} else {
		value.parse::<Balance>().expect("invalid dollar value")
	}
}

fn parse_price(value: Option<&String>) -> FixedU128 {
	FixedU128::from_parts(parse_dollar(value))
}

fn parse_permill(value: Option<&String>) -> Permill {
	let value = value.expect("Missing percentage");
	let value = value.replace(" ", "").replace("_", "");
	if value.ends_with("%") {
		let num = value[..value.len() - 1].parse::<f64>().expect("Invalid dollar value");
		Permill::from_fraction(num / 100f64)
	} else {
		Permill::from_parts(value.parse::<u32>().expect("invalid dollar value"))
	}
}

fn parse_fixed128(value: Option<&String>) -> Fixed128 {
	let value = value.expect("Missing percentage");
	let value = value.replace(" ", "").replace("_", "");
	if value.ends_with("%") {
		let num = value[..value.len() - 1].parse::<f64>().expect("Invalid dollar value");
		Fixed128::from_parts((num / 100f64 * (10u64.pow(18) as f64)) as i128)
	} else {
		Fixed128::from_parts(value.parse::<i128>().expect("invalid dollar value"))
	}
}

fn parse_currency(name: Option<&String>) -> CurrencyId {
	let name = name.expect("Missing name");
	match name.to_ascii_lowercase().trim() {
		"ausd" => CurrencyId::AUSD,
		"feur" => CurrencyId::FEUR,
		"fjpy" => CurrencyId::FJPY,
		"fbet" => CurrencyId::FBTC,
		"feth" => CurrencyId::FETH,
		_ => panic!("Invalid currency"),
	}
}

fn parse_pair(name: Option<&String>) -> TradingPair {
	let name = name.expect("Missing name");
	match name
		.to_ascii_lowercase()
		.replace(" ", "")
		.replace("_", "")
		.replace("/", "")
		.as_str()
	{
		"eurusd" => EUR_USD,
		"jpyeur" => JPY_EUR,
		_ => panic!("Invalid pair"),
	}
}

fn parse_block_number(value: Option<&String>) -> BlockNumber {
	let value = value.expect("Missing block number");
	value.trim().parse().expect("Invalid block number")
}

fn parse_leverage(leverage: Option<&String>) -> Leverage {
	let leverage = leverage.expect("Missing leverage");
	match leverage.to_ascii_lowercase().trim() {
		"long 2" => Leverage::LongTwo,
		"long 3" => Leverage::LongThree,
		"long 5" => Leverage::LongFive,
		"long 10" => Leverage::LongTen,
		"long 20" => Leverage::LongTwenty,
		"long 30" => Leverage::LongThirty,
		"long 50" => Leverage::LongFifty,
		"short 2" => Leverage::ShortTwo,
		"short 3" => Leverage::ShortThree,
		"short 5" => Leverage::ShortFive,
		"short 10" => Leverage::ShortTen,
		"short 20" => Leverage::ShortTwenty,
		"short 30" => Leverage::ShortThirty,
		"short 50" => Leverage::ShortFifty,
		_ => panic!("Unsupported leverage"),
	}
}

fn parse_position_id(value: Option<&String>) -> PositionId {
	let value = value.expect("Missing position ID");
	value.trim().parse().expect("Invalid position ID")
}

enum AssertResult {
	Ok,
	Error(String),
}

fn parse_result(value: Option<&String>) -> AssertResult {
	match value {
		Some(x) if x.trim().to_ascii_lowercase() == "ok" => AssertResult::Ok,
		Some(x) => AssertResult::Error(x.trim().into()),
		None => AssertResult::Ok,
	}
}

impl AssertResult {
	fn assert(&self, actual: DispatchResult) {
		match self {
			AssertResult::Ok => assert_ok!(actual),
			AssertResult::Error(x) => {
				assert_noop!(actual.map_err(|x| -> &str { x.into() }), x.as_str());
			}
		};
	}
}

mod steps {
	use super::*;
	use cucumber::{typed_regex, Step, Steps, StepsBuilder};

	fn get_rows(step: &Step) -> &Vec<Vec<String>> {
		&step.table.as_ref().expect("require a table").rows
	}

	pub fn steps() -> Steps<crate::World> {
		let mut builder: StepsBuilder<crate::World> = StepsBuilder::new();

		builder
			.given("accounts", |world, step| {
				world.ext = Some(
					ExtBuilder::default()
						.balances(
							get_rows(step)
								.iter()
								.map(|x| (parse_name(x.get(0)), CurrencyId::AUSD, parse_dollar(x.get(1))))
								.collect::<Vec<_>>(),
						)
						.build(),
				);
			})
			.given("create liquidity pool", |world, _step| {
				world.execute_with(|| {
					assert_ok!(create_pool());
					assert_ok!(set_enabled_trades());
				});
			})
			.given("margin deposit liquidity", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_dollar(x.get(1)), parse_result(x.get(2))));
					for (account, amount, expected) in iter {
						expected.assert(margin_deposit_liquidity(&account, amount));
					}
				})
			})
			.given("margin deposit", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_dollar(x.get(1)), parse_result(x.get(2))));
					for (account, amount, expected) in iter {
						expected.assert(margin_deposit(&account, amount));
					}
				})
			})
			.given("oracle price", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_currency(x.get(0)), parse_price(x.get(1))));
					assert_ok!(set_oracle_price(iter.collect()));
				})
			})
			.given("margin spread", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_pair(x.get(0)), parse_permill(x.get(1))));
					for (pair, value) in iter {
						assert_ok!(margin_set_spread(pair, value));
					}
				})
			})
			.given("margin set accumulate", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_pair(x.get(0)),
							parse_block_number(x.get(1)),
							parse_block_number(x.get(2)),
						)
					});
					for (pair, frequency, offset) in iter {
						assert_ok!(margin_set_accumulate(pair, frequency, offset));
					}
				})
			})
			.given("margin update swap", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
<<<<<<< HEAD
						.map(|x| (parse_pair(x.get(0)), parse_fixed128(x.get(1)), parse_fixed128(x.get(2))));
					for (pair, long, short) in iter {
						assert_ok!(margin_set_swap_rate(pair, long, short));
=======
						.map(|x| (parse_pair(x.get(0)), parse_fixed128(x.get(1))));
					for (pair, value) in iter {
						assert_ok!(margin_update_swap(pair, value));
>>>>>>> add cucumber tests
					}
				})
			})
			.given_regex(
				r"margin set min leveraged amount to (\$?[\W\d]+)",
				|world, matches, step| {
					world.execute_with(|| {
						let amount = parse_dollar(matches.get(1));
						assert_ok!(margin_set_min_leveraged_amount(amount));
					})
				},
			)
			.given_regex(
				r"margin set default min leveraged amount to (\$?[\W\d]+)",
				|world, matches, step| {
					world.execute_with(|| {
						let amount = parse_dollar(matches.get(1));
						assert_ok!(margin_set_default_min_leveraged_amount(amount));
					})
				},
			)
			.given_regex(r"margin enable trading pair (.+)", |world, matches, step| {
				world.execute_with(|| {
					let pair = parse_pair(matches.get(1));
					assert_ok!(margin_enable_trading_pair(pair));
					assert_ok!(margin_liquidity_pool_enable_trading_pair(pair));
				})
			})
			.when("open positions", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name((x.get(0))),
							parse_pair(x.get(1)),
							parse_leverage(x.get(2)),
							parse_dollar(x.get(3)),
							parse_price(x.get(4)),
							parse_result(x.get(5)),
						)
					});
					for (name, pair, leverage, amount, price, result) in iter {
						result.assert(margin_open_position(&name, pair, leverage, amount, price));
					}
				})
			})
			.when("close positions", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name((x.get(0))),
							parse_position_id(x.get(1)),
							parse_price(x.get(2)),
							parse_result(x.get(3)),
						)
					});
					for (name, position_id, price, result) in iter {
						result.assert(margin_close_position(&name, position_id, price));
					}
				})
			})
			.when("withdraw", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name((x.get(0))), parse_dollar(x.get(1)), parse_result(x.get(2))));
					for (name, amount, result) in iter {
						result.assert(margin_withdraw(&name, amount));
					}
				})
			})
			.then("balances are", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name((x.get(0))), parse_dollar(x.get(1)), parse_dollar(x.get(2))));
					for (name, free, margin) in iter {
						assert_eq!(collateral_balance(&name), free);
						assert_eq!(margin_balance(&name), margin);
					}
				})
			})
			.then_regex(r"margin liquidity is (\$?[\W\d]+)", |world, matches, step| {
				world.execute_with(|| {
					let amount = parse_dollar(matches.get(1));
					assert_eq!(margin_liquidity(), amount);
				})
			});

		builder.build()
	}
<<<<<<< HEAD
}

=======

	// Any type that implements cucumber::World + Default can be the world
	// steps!(crate::World => {
	//     given "I am trying out Cucumber" |world, _step| {
	//         world.foo = "Some string".to_string();
	//         // Set up your context in given steps
	//     };

	//     when "I consider what I am doing" |world, _step| {
	//         // Take actions
	//         let new_string = format!("{}.", &world.foo);
	//         world.foo = new_string;
	//     };

	//     then "I am interested in ATDD" |world, _step| {
	//         // Check that the outcomes to be observed have occurred
	//         assert_eq!(world.foo, "Some string.");
	//     };

	//     then regex r"^we can (.*) rules with regex$" |_world, matches, _step| {
	//         // And access them as an array
	//         assert_eq!(matches[1], "implement");
	//     };

	//     then regex r"^we can also match (\d+) (.+) types$" (usize, String) |_world, num, word, _step| {
	//         // `num` will be of type usize, `word` of type String
	//         assert_eq!(num, 42);
	//         assert_eq!(word, "olika");
	//     };

	//     then "we can use data tables to provide more parameters" |_world, step| {
	//         let table = step.table().unwrap().clone();

	//         assert_eq!(table.header, vec!["key", "value"]);

	//         let expected_keys = table.rows.iter().map(|row| row[0].to_owned()).collect::<Vec<_>>();
	//         let expected_values = table.rows.iter().map(|row| row[1].to_owned()).collect::<Vec<_>>();

	//         assert_eq!(expected_keys, vec!["a", "b"]);
	//         assert_eq!(expected_values, vec!["fizz", "buzz"]);
	//     };
	// });
}

// Declares a before handler function named `a_before_fn`
before!(a_before_fn => |_scenario| {

});

// Declares an after handler function named `an_after_fn`
after!(an_after_fn => |_scenario| {

});

// A setup function to be called before everything else
fn setup() {}

>>>>>>> add cucumber tests
cucumber! {
	features: "./features", // Path to our feature files
	world: ::World, // The world needs to be the same for steps and the main cucumber call
	steps: &[
		steps::steps // the `steps!` macro creates a `steps` function in a module
<<<<<<< HEAD
=======
	],
	setup: setup, // Optional; called once before everything
	before: &[
		a_before_fn // Optional; called before each scenario
	],
	after: &[
		an_after_fn // Optional; called after each scenario
>>>>>>> add cucumber tests
	]
}
