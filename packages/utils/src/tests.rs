use std::str::FromStr;

use cosmwasm_std::{Decimal, Uint128};

use crate::helpers::calculate_compound_amounts;

#[test]
fn calculate_compound_amounts_even_test() {
    let percentages = vec![
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.25").unwrap(),
    ];

    let total_amount = Uint128::from(100u128);

    let expected_amounts = vec![
        Uint128::from(25u128),
        Uint128::from(25u128),
        Uint128::from(25u128),
        Uint128::from(25u128),
    ];

    let amounts = calculate_compound_amounts(&percentages, &total_amount).unwrap();

    assert_eq!(amounts, expected_amounts);
}

#[test]
fn calculate_compound_amounts_with_remainder_test() {
    let percentages = vec![
        Decimal::from_str("0.25").unwrap(),
        Decimal::from_str("0.3333").unwrap(),
        Decimal::from_str("0.3333").unwrap(),
        Decimal::from_str("0.0834").unwrap(),
    ];

    let total_amount = Uint128::from(100u128);

    let expected_amounts = vec![
        Uint128::from(25u128),
        Uint128::from(33u128),
        Uint128::from(33u128),
        Uint128::from(9u128),
    ];

    let amounts = calculate_compound_amounts(&percentages, &total_amount).unwrap();

    assert_eq!(amounts, expected_amounts);
}
#[test]
fn calculate_compound_amounts_with_multiple_remainder_test() {
    let percentages = vec![
        Decimal::from_str("0.1666666666").unwrap(),
        Decimal::from_str("0.1666666666").unwrap(),
        Decimal::from_str("0.1666666667").unwrap(),
        Decimal::from_str("0.1666666667").unwrap(),
        Decimal::from_str("0.1666666667").unwrap(),
        Decimal::from_str("0.1666666667").unwrap(),
    ];

    let total_amount = Uint128::from(100u128);

    let expected_amounts = vec![
        Uint128::from(16u128),
        Uint128::from(16u128),
        Uint128::from(16u128),
        Uint128::from(16u128),
        Uint128::from(16u128),
        Uint128::from(20u128),
    ];

    let amounts = calculate_compound_amounts(&percentages, &total_amount).unwrap();

    assert_eq!(amounts, expected_amounts);
}
