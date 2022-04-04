use chrono::prelude::*;
use std::collections::HashMap;

use crate::bitcoin_parser::{BitcoinTransaction};

mod rates_table;

pub struct ExchangeRates {
	rates_table: HashMap<chrono::Date<Utc>, f64>
}

impl ExchangeRates {
	pub fn new() -> ExchangeRates {
		ExchangeRates {
			rates_table: rates_table::get_rates_table()
		}
	}
}

impl BitcoinTransaction {
	pub fn value_usd(self: &BitcoinTransaction, rates: &ExchangeRates) -> Option<f64> {
		rates.rates_table.get(&self.timestamp.date()).map(|r| r * self.value() as f64)
	}
}
