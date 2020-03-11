//! Unit tests for the margin protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{ExtBuilder, Runtime};

#[test]
fn test() {
	ExtBuilder::default().build().execute_with(|| {});
}
