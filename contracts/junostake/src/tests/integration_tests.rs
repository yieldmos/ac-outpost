use cosmwasm_std::{coin, coins, testing::mock_env, Addr, CosmosMsg, Decimal, Delegation, Empty, Validator};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, StakingInfo};
use outpost_utils::juno_comp_prefs::{
    DaoAddress, DaoAddresses, DestinationProjectAddresses, GelottoAddresses, RacoonBetAddresses, SparkIbcAddresses,
    WhiteWhaleSatelliteAddresses, WyndAddresses,
};

use crate::{
    contract::{execute, instantiate, query},
    msg::{AuthzppAddresses, ContractAddresses, InstantiateMsg},
    tests::multitest::OutpostContract,
};

fn auctioning_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);

    Box::new(contract)
}

#[test]
fn instantiate_with_defaults() {
    let sender = Addr::unchecked("sender");

    let mut app = App::new(|_router, _api, _storage| {
        // router.bank
        // .init_balance(storage, &sender, coins(100_000, "ubtc"))
        // .unwrap();
    });

    let contract_id = app.store_code(auctioning_contract());

    let _contract = OutpostContract::instantiate(
        &mut app,
        contract_id,
        &sender,
        None,
        "Test Outpost",
        // &coins(100_000, "ubtc"),
        &InstantiateMsg {
            admin: None,
            project_addresses: ContractAddresses {
                usdc: wyndex::asset::AssetInfo::Native("".to_string()),
                authzpp: AuthzppAddresses {
                    withdraw_tax: "".to_string(),
                },
                destination_projects: DestinationProjectAddresses {
                    wynd: WyndAddresses {
                        cw20: "".to_string(),
                        multihop: "".to_string(),
                        juno_wynd_pair: "".to_string(),
                    },
                    gelotto: GelottoAddresses {
                        pick3_contract: "".to_string(),
                        pick4_contract: "".to_string(),
                        pick5_contract: "".to_string(),
                    },
                    daos: DaoAddresses {
                        neta: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        signal: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        posthuman: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        kleomedes: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        cannalabs: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        muse: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                    },
                    spark_ibc: SparkIbcAddresses { fund: "".to_string() },
                    balance_dao: "".to_string(),
                    white_whale: WhiteWhaleSatelliteAddresses {
                        amp_whale: wyndex::asset::AssetInfo::Native("".to_string()),
                        bone_whale: wyndex::asset::AssetInfo::Native("".to_string()),
                        market: "".to_string(),
                        rewards: "".to_string(),
                    },
                    racoon_bet: RacoonBetAddresses {
                        game: "".to_string(),
                        juno_usdc_wynd_pair: "".to_string(),
                    },
                    juno_lsds: outpost_utils::juno_comp_prefs::JunoLsdAddresses {
                        bone_juno: "".to_string(),
                        wy_juno: "".to_string(),
                        se_juno: "".to_string(),
                        b_juno: "".to_string(),
                        amp_juno: "".to_string(),
                    },
                },
                take_rate_addr: "".to_string(),
            },
        },
    )
    .unwrap();

    assert_eq!(app.wrap().query_all_balances(&sender).unwrap(), &[])
}

#[test]
fn validator_only_compounding() {
    let contract_admin_addr = Addr::unchecked("admin_wallet");
    let delegator_addr = Addr::unchecked("delegator_wallet");
    let start_validator_addr = Addr::unchecked("validator_1");
    let end_validator_addr = Addr::unchecked("validator_2");

    let mut app = App::new(|router, api, storage| {
        router
            .bank
            .init_balance(storage, &delegator_addr, coins(100_000, "ubtc"))
            .unwrap();

        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: "ubtc".to_string(),
                    unbonding_time: 10u64,
                    apr: Decimal::percent(1000),
                },
            )
            .unwrap();

        router
            .staking
            .add_validator(
                api,
                storage,
                &mock_env().block,
                Validator {
                    address: start_validator_addr.to_string(),
                    commission: Decimal::one(),
                    max_commission: Decimal::one(),
                    max_change_rate: Decimal::one(),
                },
            )
            .unwrap();

        router
            .staking
            .add_validator(
                api,
                storage,
                &mock_env().block,
                Validator {
                    address: end_validator_addr.to_string(),
                    commission: Decimal::one(),
                    max_commission: Decimal::one(),
                    max_change_rate: Decimal::one(),
                },
            )
            .unwrap();
    });

    let contract_id = app.store_code(auctioning_contract());

    let _contract = OutpostContract::instantiate(
        &mut app,
        contract_id,
        &contract_admin_addr,
        None,
        "Test Outpost",
        &InstantiateMsg {
            admin: None,
            project_addresses: ContractAddresses {
                take_rate_addr: "".to_string(),
                usdc: wyndex::asset::AssetInfo::Native("".to_string()),
                authzpp: AuthzppAddresses {
                    withdraw_tax: "".to_string(),
                },
                destination_projects: DestinationProjectAddresses {
                    wynd: WyndAddresses {
                        cw20: "".to_string(),
                        multihop: "".to_string(),
                        juno_wynd_pair: "".to_string(),
                    },
                    gelotto: GelottoAddresses {
                        pick3_contract: "".to_string(),
                        pick4_contract: "".to_string(),
                        pick5_contract: "".to_string(),
                    },
                    daos: DaoAddresses {
                        neta: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        signal: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        posthuman: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        kleomedes: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        cannalabs: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                        muse: DaoAddress {
                            cw20: "".to_string(),
                            staking: "".to_string(),
                            juno_wyndex_pair: Some("".to_string()),
                            wynd_wyndex_pair: None,
                        },
                    },
                    spark_ibc: SparkIbcAddresses { fund: "".to_string() },
                    balance_dao: "".to_string(),
                    white_whale: WhiteWhaleSatelliteAddresses {
                        amp_whale: wyndex::asset::AssetInfo::Native("".to_string()),
                        bone_whale: wyndex::asset::AssetInfo::Native("".to_string()),
                        market: "".to_string(),
                        rewards: "".to_string(),
                    },
                    racoon_bet: RacoonBetAddresses {
                        game: "".to_string(),
                        juno_usdc_wynd_pair: "".to_string(),
                    },
                    juno_lsds: outpost_utils::juno_comp_prefs::JunoLsdAddresses {
                        bone_juno: "".to_string(),
                        wy_juno: "".to_string(),
                        se_juno: "".to_string(),
                        b_juno: "".to_string(),
                        amp_juno: "".to_string(),
                    },
                },
            },
        },
    )
    .unwrap();

    app.execute_multi(
        delegator_addr.clone(),
        vec![
            CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
                validator: start_validator_addr.to_string(),
                amount: coin(100_000, "ubtc"),
            }),
            // create_generic_grant_msg(
            //     delegator_addr.to_string(),
            //     &contract.addr(),
            //     GenericAuthorizationType::Delegation,
            // ),
            // create_generic_grant_msg(
            //     delegator_addr.to_string(),
            //     &contract.addr(),
            //     GenericAuthorizationType::WithdrawDelegatorRewards,
            // ),
        ],
    )
    .unwrap();

    assert_eq!(app.wrap().query_all_balances(&delegator_addr).unwrap(), &[]);
    assert_eq!(
        app.wrap().query_all_delegations(delegator_addr.clone()).unwrap(),
        vec![Delegation {
            delegator: delegator_addr.clone(),
            validator: start_validator_addr.to_string(),
            amount: coin(100_000, "ubtc".to_string())
        }]
    );

    app.update_block(next_block);

    // let err = contract
    //     .compound_funds(
    //         &mut app,
    //         &delegator_addr.clone(),
    //         CompoundPrefs {
    //             relative: vec![DestinationAction {
    //                 destination: DestinationProject::JunoStaking {
    //                     validator_address: end_validator_addr.to_string(),
    //                 },
    //                 amount: RelativeQty {
    //                     quantity: 1_000_000_000_000_000_000u128.into(),
    //                 },
    //             }],
    //         },
    //         delegator_addr.to_string(),
    //     )
    //     .unwrap_err();

    // println!("{}", err)
}
