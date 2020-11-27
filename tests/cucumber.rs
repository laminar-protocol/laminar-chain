#![cfg(test)]

use cucumber::cucumber;

use dev_runtime::{tests::*, AccountId, CurrencyId, Moment};
use frame_support::{assert_noop, assert_ok};
use laminar_primitives::{Balance, Leverage, TradingPair};
use margin_protocol::RiskThreshold;
use margin_protocol_rpc_runtime_api::{MarginPoolState, MarginTraderState};
use sp_arithmetic::{FixedI128, FixedU128};
use sp_runtime::{traits::Bounded, DispatchResult, Permill};
use std::ops::Range;
use synthetic_protocol_rpc_runtime_api::SyntheticPoolState;

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
		// to avoid accuracy issue when doing conversion
		((num * (10u64.pow(10) as f64)) as Balance) * 10u128.pow(8)
	} else {
		value.parse::<Balance>().expect("invalid dollar value")
	}
}

fn parse_fixed_i128_dollar(value: Option<&String>) -> FixedI128 {
	let value = value.expect("Missing balance");
	let value = value.replace(" ", "").replace("_", "");
	let dollar = if value.starts_with("$") {
		let num = value[1..].parse::<f64>().expect("Invalid dollar value");
		// to avoid accuracy issue when doing conversion
		((num * (10u64.pow(10) as f64)) as i128) * 10i128.pow(8)
	} else {
		value.parse::<i128>().expect("invalid dollar value")
	};
	FixedI128::from_inner(dollar)
}

fn parse_price(value: Option<&String>) -> FixedU128 {
	FixedU128::from_inner(parse_dollar(value))
}

fn parse_bool(value: Option<&String>) -> bool {
	let value = value.expect("Missing bool");
	match value.to_ascii_lowercase().trim() {
		"true" => true,
		"false" => false,
		_ => panic!("Invalid bool value"),
	}
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

fn parse_fixedi128(value: Option<&String>) -> FixedI128 {
	let value = value.expect("Missing percentage");
	let value = value.replace(" ", "").replace("_", "");
	if value.ends_with("%") {
		let num = value[..value.len() - 1].parse::<f64>().expect("Invalid dollar value");
		FixedI128::from_inner((num / 100f64 * (10u64.pow(18) as f64)) as i128)
	} else if value == "MaxValue" {
		FixedI128::max_value()
	} else {
		FixedI128::from_inner(value.parse::<i128>().expect("invalid dollar value"))
	}
}

fn parse_fixed_u128(value: Option<&String>) -> FixedU128 {
	let value = value.expect("Missing percentage");
	let value = value.replace(" ", "").replace("_", "");
	if value.ends_with("%") {
		let num = value[..value.len() - 1].parse::<f64>().expect("Invalid dollar value");
		FixedU128::from_inner((num / 100f64 * (10u64.pow(18) as f64)) as u128)
	} else {
		FixedU128::from_inner(value.parse::<u128>().expect("invalid dollar value"))
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
		"jpyusd" => JPY_USD,
		_ => panic!("Invalid pair"),
	}
}

fn parse_time(value: Option<&String>) -> Moment {
	let value = value.expect("Missing time");
	if value.ends_with("s") {
		value[..value.len() - 1].parse::<u64>().expect("Invalid time")
	} else if value.ends_with("min") {
		value[..value.len() - 3].parse::<u64>().expect("Invalid time") * 60
	} else if value.ends_with("h") {
		value[..value.len() - 1].parse::<u64>().expect("Invalid time") * 60 * 60
	} else {
		panic!("Invalid time format");
	}
}

fn parse_time_range(value: Option<&String>) -> Range<Moment> {
	let value = value.expect("Missing time");
	let range: Vec<&str> = value.trim().split("..").collect();
	let start = range[0];
	let end = range[1];

	if start.ends_with("s") {
		let start = start[..start.len() - 1].parse::<u64>().expect("Invalid time");
		let end = end[..end.len() - 1].parse::<u64>().expect("Invalid time");
		Range { start: start, end: end }
	} else if start.ends_with("min") {
		let start = start[..start.len() - 3].parse::<u64>().expect("Invalid time");
		let end = end[..end.len() - 3].parse::<u64>().expect("Invalid time");
		Range {
			start: start * 60,
			end: end * 60,
		}
	} else if start.ends_with("h") {
		let start = start[..start.len() - 1].parse::<u64>().expect("Invalid time");
		let end = end[..end.len() - 1].parse::<u64>().expect("Invalid time");
		Range {
			start: start * 60 * 60,
			end: end * 60 * 60,
		}
	} else {
		panic!("Invalid time format");
	}
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

fn parse_threshold(value: Option<&String>) -> Option<RiskThreshold> {
	let value = value.expect("Missing threshold");
	let value = value.replace("(", "").replace(")", "").replace(" ", "");
	let threshold = value
		.trim()
		.split(",")
		.map(|value| {
			if value.ends_with("%") {
				let num = value[..value.len() - 1].parse::<u32>().expect("Invalid threshold");
				Permill::from_percent(num)
			} else {
				let num = value.parse::<u32>().expect("Invalid threshold");
				Permill::from_parts(num)
			}
		})
		.collect::<Vec<Permill>>();

	Some(RiskThreshold {
		margin_call: threshold[0],
		stop_out: threshold[1],
	})
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
			AssertResult::Ok => {
				assert_ok!(actual);
			}
			AssertResult::Error(x) => {
				assert_noop!(actual.map_err(|x| -> &str { x.into() }), x.as_str());
			}
		};
	}
}

mod steps {
	use super::*;
	use cucumber::{Step, Steps, StepsBuilder};

	fn get_rows(step: &Step) -> &Vec<Vec<String>> {
		&step.table.as_ref().expect("require a table").rows
	}

	pub fn margin_steps() -> Steps<crate::World> {
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
			.given("margin create liquidity pool", |world, _step| {
				world.execute_with(|| {
					assert_ok!(margin_create_pool());
					assert_ok!(margin_set_enabled_trades());
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
						.map(|x| (parse_pair(x.get(0)), parse_price(x.get(1))));
					for (pair, value) in iter {
						assert_ok!(margin_set_spread(pair, value));
					}
				})
			})
			.given("margin set accumulate", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_pair(x.get(0)), parse_time(x.get(1)), parse_time(x.get(2))));
					for (pair, frequency, offset) in iter {
						assert_ok!(margin_set_accumulate(pair, frequency, offset));
					}
				})
			})
			.given("margin set swap rate", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_pair(x.get(0)),
							parse_fixedi128(x.get(1)),
							parse_fixedi128(x.get(2)),
						)
					});
					for (pair, long, short) in iter {
						assert_ok!(margin_set_swap_rate(pair, long, short));
					}
				})
			})
			.given_regex(
				r"margin set min leveraged amount to (\$?[\W\d_]+)",
				|world, matches, _step| {
					world.execute_with(|| {
						let amount = parse_dollar(matches.get(1));
						assert_ok!(margin_set_min_leveraged_amount(amount));
					})
				},
			)
			.given_regex(
				r"margin set default min leveraged amount to (\$?[\W\d_]+)",
				|world, matches, _step| {
					world.execute_with(|| {
						let amount = parse_dollar(matches.get(1));
						assert_ok!(margin_set_default_min_leveraged_amount(amount));
					})
				},
			)
			.given_regex(r"margin enable trading pair (.+)", |world, matches, _step| {
				world.execute_with(|| {
					let pair = parse_pair(matches.get(1));
					assert_ok!(margin_enable_trading_pair(pair));
					assert_ok!(margin_liquidity_pool_enable_trading_pair(pair));
				})
			})
			.given("margin set risk threshold(margin_call, stop_out)", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_pair(x.get(0)),
							parse_threshold(x.get(1)),
							parse_threshold(x.get(2)),
							parse_threshold(x.get(3)),
						)
					});
					for (pair, trader, enp, ell) in iter {
						assert_ok!(margin_set_risk_threshold(pair, trader, enp, ell));
					}
				})
			})
			.when("open positions", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
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
			.then("oracle price", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_currency(x.get(0)), parse_price(x.get(1))));
					assert_ok!(set_oracle_price(iter.collect()));
				})
			})
			.then("margin set risk threshold(margin_call, stop_out)", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_pair(x.get(0)),
							parse_threshold(x.get(1)),
							parse_threshold(x.get(2)),
							parse_threshold(x.get(3)),
						)
					});
					for (pair, trader, enp, ell) in iter {
						assert_ok!(margin_set_risk_threshold(pair, trader, enp, ell));
					}
				})
			})
			.then("margin trader margin call", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_result(x.get(1))));
					for (name, result) in iter {
						result.assert(margin_trader_margin_call(&name));
					}
				})
			})
			.then("margin trader liquidate", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_result(x.get(1))));
					for (name, result) in iter {
						result.assert(margin_trader_stop_out(&name));
					}
				})
			})
			.then("margin trader become safe", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_result(x.get(1))));
					for (name, result) in iter {
						result.assert(margin_trader_become_safe(&name));
					}
				})
			})
			.then("margin liquidity pool margin call", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| parse_result(x.get(0)));
					for result in iter {
						result.assert(margin_liquidity_pool_margin_call());
					}
				})
			})
			.then("margin liquidity pool liquidate", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| parse_result(x.get(0)));
					for result in iter {
						result.assert(margin_liquidity_pool_force_close());
					}
				})
			})
			.then("margin liquidity pool become safe", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| parse_result(x.get(0)));
					for result in iter {
						result.assert(margin_liquidity_pool_become_safe());
					}
				})
			})
			.when("close positions", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
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
			.when("margin withdraw", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_dollar(x.get(1)), parse_result(x.get(2))));
					for (name, amount, result) in iter {
						result.assert(margin_withdraw(&name, amount));
					}
				})
			})
			.then("margin balances are", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
							parse_dollar(x.get(1)),
							parse_fixed_i128_dollar(x.get(2)),
						)
					});
					for (name, free, margin) in iter {
						assert_eq!(collateral_balance(&name), free);
						assert_eq!(margin_balance(&name), margin);
					}
				})
			})
			.then_regex(r"margin liquidity is (\$?[\W\d_]+)", |world, matches, _step| {
				world.execute_with(|| {
					let amount = parse_dollar(matches.get(1));
					assert_eq!(margin_liquidity(), amount);
				})
			})
			.then_regex(r"margin execute time ([\d\w]+..[\d\w]+)", |world, matches, _step| {
				world.execute_with(|| {
					let time_range = parse_time_range(matches.get(1));
					margin_execute_time(time_range);
				})
			})
			.then_regex(r"margin set additional swap (.+)", |world, matches, _step| {
				world.execute_with(|| {
					let swap = parse_fixedi128(matches.get(1));
					assert_ok!(margin_set_additional_swap(swap));
				})
			})
			.then("margin trader info are", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
							parse_fixed_i128_dollar(x.get(1)),
							parse_fixed_i128_dollar(x.get(2)),
							parse_fixedi128(x.get(3)),
							parse_fixed_i128_dollar(x.get(4)),
							parse_fixed_i128_dollar(x.get(5)),
						)
					});
					for (name, equity, margin_held, margin_level, free_margin, unrealized_pl) in iter {
						assert_eq!(
							margin_trader_state(&name),
							MarginTraderState {
								equity: equity,
								margin_held: margin_held,
								margin_level: margin_level,
								free_margin: free_margin,
								unrealized_pl: unrealized_pl,
							}
						);
					}
				})
			})
			.then("margin pool info are", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_fixedi128(x.get(0)),
							parse_fixedi128(x.get(1)),
							parse_fixed_i128_dollar(x.get(2)),
						)
					});
					for (enp, ell, required_deposit) in iter {
						assert_eq!(
							margin_pool_state(),
							Some(MarginPoolState {
								enp: enp,
								ell: ell,
								required_deposit: required_deposit,
							})
						);
					}
				})
			})
			.then_regex(r"treasury balance is (\$?[\W\d_]+)", |world, matches, _step| {
				world.execute_with(|| {
					let amount = parse_dollar(matches.get(1));
					assert_eq!(treasury_balance(), amount);
				})
			});

		builder.build()
	}

	pub fn synthetic_steps() -> Steps<crate::World> {
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
			.given("synthetic create liquidity pool", |world, _step| {
				world.execute_with(|| {
					assert_ok!(synthetic_create_pool());
					assert_ok!(synthetic_set_enabled_trades());
				});
			})
			.given("synthetic deposit liquidity", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_name(x.get(0)), parse_dollar(x.get(1)), parse_result(x.get(2))));
					for (account, amount, expected) in iter {
						expected.assert(synthetic_deposit_liquidity(&account, amount));
					}
				})
			})
			.given_regex(
				r"synthetic set min additional collateral ratio to (\$?[\W\d_]+)",
				|world, matches, _step| {
					world.execute_with(|| {
						let ratio = parse_permill(matches.get(1));
						assert_ok!(synthetic_set_min_additional_collateral_ratio(ratio));
					})
				},
			)
			.given("synthetic set additional collateral ratio", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_currency(x.get(0)), parse_permill(x.get(1))));
					for (currency, ratio) in iter {
						assert_ok!(synthetic_set_additional_collateral_ratio(currency, ratio));
					}
				})
			})
			.given("synthetic set spread", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step)
						.iter()
						.map(|x| (parse_currency(x.get(0)), parse_price(x.get(1))));
					for (currency, spread) in iter {
						assert_ok!(synthetic_set_spread(currency, spread));
					}
				})
			})
			.when("synthetic buy", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
							parse_currency(x.get(1)),
							parse_dollar(x.get(2)),
							parse_result(x.get(3)),
						)
					});
					for (name, currency, amount, result) in iter {
						result.assert(synthetic_buy(&name, currency, amount));
					}
				})
			})
			.when("synthetic sell", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
							parse_currency(x.get(1)),
							parse_dollar(x.get(2)),
							parse_result(x.get(3)),
						)
					});
					for (name, currency, amount, result) in iter {
						result.assert(synthetic_sell(&name, currency, amount));
					}
				})
			})
			.then("synthetic balances are", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_name(x.get(0)),
							parse_dollar(x.get(1)),
							parse_currency(x.get(2)),
							parse_dollar(x.get(3)),
						)
					});
					for (name, free, currency, synthetic) in iter {
						assert_eq!(collateral_balance(&name), free);
						assert_eq!(multi_currency_balance(&name, currency), synthetic);
					}
				})
			})
			.then("synthetic pool info are", |world, step| {
				world.execute_with(|| {
					let iter = get_rows(step).iter().map(|x| {
						(
							parse_currency(x.get(0)),
							parse_fixed_u128(x.get(1)),
							parse_bool(x.get(2)),
						)
					});
					for (currency_id, collateral_ratio, is_safe) in iter {
						assert_eq!(
							synthetic_pool_state(currency_id),
							Some(SyntheticPoolState {
								collateral_ratio: collateral_ratio,
								is_safe: is_safe,
							})
						);
					}
				})
			})
			.then_regex(r"synthetic module balance is (\$?[\W\d_]+)", |world, matches, _step| {
				world.execute_with(|| {
					let amount = parse_dollar(matches.get(1));
					assert_eq!(synthetic_balance(), amount);
				})
			})
			.then_regex(r"synthetic liquidity is (\$?[\W\d_]+)", |world, matches, _step| {
				world.execute_with(|| {
					let amount = parse_dollar(matches.get(1));
					assert_eq!(synthetic_liquidity(), amount);
				})
			});

		builder.build()
	}
}

cucumber! {
	features: "./features", // Path to our feature files
	world: ::World, // The world needs to be the same for steps and the main cucumber call
	steps: &[
		steps::margin_steps, // the `steps!` macro creates a `steps` function in a module
		steps::synthetic_steps, // the `steps!` macro creates a `steps` function in a module
	]
}
