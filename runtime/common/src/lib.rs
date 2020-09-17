//! Common runtime code

#![cfg_attr(not(feature = "std"), no_std)]

pub type TimeStampedPrice = orml_oracle::TimestampedValue<primitives::Price, primitives::Moment>;
