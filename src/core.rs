use linked_hash_map::LinkedHashMap;
use std::ops::{Add, Mul, Neg};
use bulletproofs::r1cs::LinearCombination;
use curve25519_dalek::scalar::Scalar;
use crate::constants::INTEGERIZE_FACTOR;
use crate::model::PriceMap;

pub trait ZKPNLCalculable<LC, S>: Add<Output=LC> + Mul<S, Output=LC> + Neg<Output=LC> + Default + Clone {}

impl ZKPNLCalculable<i64, i64> for i64 {}

impl ZKPNLCalculable<LinearCombination, Scalar> for LinearCombination {}

pub trait Integerize {
    fn integerize(float: f64) -> Self;
}

pub fn inherit_portfolio<LC, S>(p: &LinkedHashMap<String, LC>, trade_map: &mut LinkedHashMap<String, Vec<(f64, LC)>>, price_map: &PriceMap)
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    for (symbol, size) in p {
        let trades = trade_map.entry(symbol.clone()).or_insert(vec![]);
        trades.push((price_map[symbol], size.clone()))
    }
}

pub fn calc_portfolio<LC, S>(trade_map: &LinkedHashMap<String, Vec<(f64, LC)>>) -> LinkedHashMap<String, LC>
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    let mut portfolio: LinkedHashMap<String, LC> = LinkedHashMap::new();
    for (symbol, trades) in trade_map {
        let size = portfolio.entry(symbol.clone()).or_insert(LC::default());
        *size = calc_size(trades);
    }
    portfolio
}

pub fn calc_size<LC, S>(trades: &[(f64, LC)]) -> LC
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    trades.iter()
        .map(|(_, qty)|qty.clone())
        .fold(LC::default(), |acc, lc|acc + lc)
}

/// long trade spends cash and short trade receives cash
/// so cash flow is the additive inverse of qty multiply price
pub fn calc_cash_balance<LC, S>(trades: &[(f64, LC)]) -> LC
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    trades.iter().map(|(price, qty)|{
        -qty.clone() * S::integerize(*price)
    }).fold(LC::default(), |acc, cash_flow|acc + cash_flow)
}

pub fn calc_market_value<LC, S>(trades: &[(f64, LC)], market_price: f64) -> LC
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    let size = trades.iter()
        .fold(LC::default(), |acc, (_, qty)|acc + qty.clone());
    size * S::integerize(market_price)
}

pub fn calc_total_pnl<LC, S>(trade_map: &LinkedHashMap<String, Vec<(f64, LC)>>, price_map: &PriceMap) -> LC
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    trade_map.iter().map(|(inst, trades)| {
        let market_price = price_map[inst];
        calc_pnl::<LC, S>(trades, market_price)
    }).fold(LC::default(), |acc, lc|acc + lc)
}

/// P&L can be described as cash balance (with an initial balance of zero)
/// plus market value of current position
fn calc_pnl<LC, S>(trades: &[(f64, LC)], market_price: f64) -> LC
    where LC: ZKPNLCalculable<LC, S>, S: Integerize {
    calc_cash_balance(trades) + calc_market_value(trades, market_price)
}

impl Integerize for i64 {
    fn integerize(float: f64) -> i64 {
        let integer = (float * INTEGERIZE_FACTOR as f64).round() as i64;
        assert_eq!(integer == 0 || integer.abs() >= 1, true);
        integer
    }
}

impl Integerize for Scalar {
    fn integerize(float: f64) -> Scalar {
        let integer = i64::integerize(float);
        if integer >= 0 {
            Scalar::from(integer as u64)
        } else {
            -Scalar::from(-integer as u64)
        }
    }
}

pub trait Deintegerize {
    fn deintegerize(&self) -> f64;
}

impl Deintegerize for i64 {
    fn deintegerize(&self) -> f64 {
        *self as f64 / INTEGERIZE_FACTOR as f64
    }
}