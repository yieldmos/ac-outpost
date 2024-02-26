use std::str::FromStr;

use cosmos_sdk_proto::cosmos::{bank::v1beta1::MsgSend, base::v1beta1::Coin};
use cosmwasm_std::{coin, Addr, Decimal, Timestamp, Uint128};

use crate::{
    helpers::{
        calc_additional_tax_split, calc_tax_split, calculate_compound_amounts,
        CompoundingFrequency, TaxSplitResult,
    },
    msg_gen::CosmosProtoMsg,
};

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

#[test]
fn test_calc_tax_split() {
    let tax_rate = Decimal::percent(1);
    let expected = Uint128::from(1_000_000u128);
    let sender = "sender".to_string();
    let receiver = "receiver".to_string();

    let split = calc_additional_tax_split(
        &coin(100_000_000, "ubtc"),
        tax_rate,
        sender.clone(),
        receiver.clone(),
    );

    assert_eq!(
        split,
        TaxSplitResult {
            remaining_rewards: coin(100_000_000, "ubtc"),
            tax_amount: coin(1_000_000, "ubtc"),
            claim_and_tax_msgs: vec![CosmosProtoMsg::Send(MsgSend {
                from_address: sender.clone(),
                to_address: receiver.clone(),
                amount: vec![Coin {
                    denom: "ubtc".to_string(),
                    amount: expected.to_string(),
                }],
            })],
        }
    );

    // with tiny amounts the extra amount goes to the taxation addr
    let split = calc_additional_tax_split(
        &coin(5, "ubtc"),
        Decimal::percent(10),
        sender.clone(),
        receiver.clone(),
    );

    assert_eq!(
        split,
        TaxSplitResult {
            remaining_rewards: coin(5, "ubtc"),
            tax_amount: coin(1, "ubtc"),
            claim_and_tax_msgs: vec![CosmosProtoMsg::Send(MsgSend {
                from_address: sender.clone(),
                to_address: receiver,
                amount: vec![Coin {
                    denom: "ubtc".to_string(),
                    amount: 1.to_string(),
                }],
            })],
        }
    );
}

#[test]
fn test_tax_split() {
    let tax_rate = Decimal::percent(1);
    let expected = Uint128::from(1_000_000u128);
    let sender = Addr::unchecked("sender");
    let receiver = Addr::unchecked("receiver");

    let split = calc_tax_split(
        &coin(100_000_000, "ubtc"),
        tax_rate,
        &sender.clone(),
        &receiver.clone(),
    );

    assert_eq!(
        split,
        TaxSplitResult {
            remaining_rewards: coin(99_000_000, "ubtc"),
            tax_amount: coin(1_000_000, "ubtc"),
            claim_and_tax_msgs: vec![CosmosProtoMsg::Send(MsgSend {
                from_address: sender.to_string(),
                to_address: receiver.to_string(),
                amount: vec![Coin {
                    denom: "ubtc".to_string(),
                    amount: expected.to_string(),
                }],
            })],
        }
    );

    // with tiny amounts the extra amount goes to the taxation addr
    let split = calc_tax_split(
        &coin(5, "ubtc"),
        Decimal::percent(10),
        &sender.clone(),
        &receiver.clone(),
    );

    assert_eq!(
        split,
        TaxSplitResult {
            remaining_rewards: coin(4, "ubtc"),
            tax_amount: coin(1, "ubtc"),
            claim_and_tax_msgs: vec![CosmosProtoMsg::Send(MsgSend {
                from_address: sender.to_string(),
                to_address: receiver.to_string(),
                amount: vec![Coin {
                    denom: "ubtc".to_string(),
                    amount: 1.to_string(),
                }],
            })],
        }
    );

    // test the split with a 0 tax rate
    let split = calc_tax_split(
        &coin(100_000_000, "ubtc"),
        Decimal::percent(0),
        &sender.clone(),
        &receiver.clone(),
    );
    assert_eq!(
        split,
        TaxSplitResult {
            remaining_rewards: coin(100_000_000, "ubtc"),
            tax_amount: coin(0, "ubtc"),
            claim_and_tax_msgs: vec![],
        }
    )
}

#[test]
fn test_compounding_freq_iteration_count() {
    let initial_time = Timestamp::from_seconds(0);

    assert_eq!(
        CompoundingFrequency::Daily
            .iteration_count(initial_time, initial_time.plus_days(1).plus_hours(2)),
        1
    );

    assert_eq!(
        CompoundingFrequency::Hourly
            .iteration_count(initial_time, initial_time.plus_days(2).plus_minutes(30)),
        48
    );

    assert_eq!(
        CompoundingFrequency::Daily
            .iteration_count(initial_time, initial_time.plus_days(365).plus_hours(2)),
        365
    );
}
