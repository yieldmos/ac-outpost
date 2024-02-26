use cosmwasm_std::{coin, coins, testing::mock_env, Addr, CosmosMsg, Decimal, Delegation, Empty, Validator};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, StakingInfo};
use juno_destinations::comp_prefs::{
    DaoAddress, DaoAddresses, DestinationProjectAddresses, GelottoAddresses, JunoLsdAddresses, RacoonBetAddresses,
    SparkIbcAddresses, WhiteWhaleSatelliteAddresses, WyndAddresses,
};

use white_whale::pool_network::{asset::AssetInfo as WWAssetInfo, router::SwapOperation};

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
        &InstantiateMsg {
            admin: None,
            project_addresses: ContractAddresses {
                staking_denom: "ujuno".to_string(),
                take_rate_addr: "juno1takerateaddr".to_string(),
                usdc: wyndex::asset::AssetInfo::Native(
                    "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
                ),
                authzpp: AuthzppAddresses {
                    withdraw_tax: "juno1nak433pjd39et4g6jjclxk7yfmtfsd5m43su04rxe9ggttdvjwpqsumv30".to_string(),
                },
                destination_projects: DestinationProjectAddresses {
                    wynd: WyndAddresses {
                        cw20: "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9".to_string(),
                        multihop: "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w".to_string(),
                        juno_wynd_pair: "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf".to_string(),
                        wynd_usdc_pair: "juno18zk9xqj9xjm0ry39jjam8qsysj7qh49xwt4qdfp9lgtrk08sd58s2n54ve".to_string(),
                    },
                    gelotto: GelottoAddresses {
                        pick3_contract: "juno1v466lyrhsflkt9anxt4wyx7mtw8w2uyk0qxkqskqfj90rmwhph7s0dxvga".to_string(),
                        pick4_contract: "juno16xy5m05z6n4vnfzcf8cvd3anxhg4g2k8vvr4q2knv4akynfstr9qjmhdhs".to_string(),
                        pick5_contract: "juno1txn3kejj4qrehua9vlg3hk4wunqafqunfy83cz5hg2xa3z3pkgssk4tzu4".to_string(),
                    },
                    daos: DaoAddresses {
                        neta: DaoAddress {
                            cw20: "juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr".to_string(),
                            staking: "juno1a7x8aj7k38vnj9edrlymkerhrl5d4ud3makmqhx6vt3dhu0d824qh038zh".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        signal: DaoAddress {
                            cw20: "juno14lycavan8gvpjn97aapzvwmsj8kyrvf644p05r0hu79namyj3ens87650k".to_string(),
                            staking: "juno1v0km8gytmzpmtnwv8mpx26kctt5szuzudhg209fxee57yh9u2cvs88rn7p".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1p3eed298qx3nyhs3grld07jrf9vjsjsmdd2kmmh3crk87emjcx5stp409y".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        posthuman: DaoAddress {
                            cw20: "juno1rws84uz7969aaa7pej303udhlkt3j9ca0l3egpcae98jwak9quzq8szn2l".to_string(),
                            staking: "juno1jktfdt5g2d0fguvy8r8pl4gly7wps8phkwy08z6upc4nazkumrwq7lj0vn".to_string(),
                            juno_wyndex_pair: Some(
                                "juno17jv00cm4f3twr548jzayu57g9txvd4zdh54mdg9qpjs7samlphjsykylsq".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        kleomedes: DaoAddress {
                            cw20: "juno10gthz5ufgrpuk5cscve2f0hjp56wgp90psqxcrqlg4m9mcu9dh8q4864xy".to_string(),
                            staking: "juno1zqp6uh3eg09s0h24rkwukkkg3pch49g0ndc53z9l8wrvh9dhf4nsj0ur49".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1dpqgt3ja2kdxs94ltjw9ncdsexts9e3dx5qpnl20zvgdguzjelhqstf8zg".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        cannalabs: DaoAddress {
                            cw20: "juno1vn38rzq0wc7zczp4dhy0h5y5kxh2jjzeahwe30c9cc6dw3lkyk5qn5rmfa".to_string(),
                            staking: "juno1066vq5g9qdprhgjst444rgf0zknhlqwmdnm7xyqhprt9whctzzxqdx90lu".to_string(),
                            juno_wyndex_pair: Some(
                                "juno17ckp36lmgtt7jtuggdv2j39eh4alcnl35szu6quh747nujags07swwq0nh".to_string(),
                            ),
                            wynd_wyndex_pair: Some(
                                "juno1ls5un4a8zyn4f05k0ekq5aa9uhn88y8362ww38elqfpcwllme0jqelamke".to_string(),
                            ),
                        },
                        muse: DaoAddress {
                            cw20: "juno1p8x807f6h222ur0vssqy3qk6mcpa40gw2pchquz5atl935t7kvyq894ne3".to_string(),
                            staking: "juno17gdhjxt2d5mhx6paxc85g4pr5myew8pq0lm7usdsavsfk34ldrsqqhtafc".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1rcssjyqgr6vzalss77d43v30c2qpyzzg607ua8gte2shqgtvu24sg8gs8r".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                    },
                    spark_ibc: SparkIbcAddresses {
                        fund: "juno1a6rna5tcl6p97rze6hnd5ug35kadqhudvr5f4mtr6s0yd5mruhss8gzrdy".to_string(),
                    },
                    balance_dao: "juno1ve7y09kvvnjk0yc2ycaq0y9thq5tct5ve6c0a5hfkt0h4jfy936qxtne5s".to_string(),
                    white_whale: WhiteWhaleSatelliteAddresses {
                        amp_whale: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF".to_string(),
                        bone_whale: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF".to_string(),
                        market: "juno1n8slcc79dmwuzdxhsesvhcncaqfg9h4czdm5t5ey8x25ajmn3xzqyde4wv".to_string(),
                        rewards: "juno184ghwgprva7dlr2hwhzgvt6mem6zx78fygk0cpw7klssmzyf67tqdtwt3h".to_string(),
                        juno_amp_whale_path: vec![
                            // swap juno for usdc
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ujuno".to_string(),
                                },

                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                            },
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
                                        .to_string(),
                                },
                            },
                        ],
                        juno_bone_whale_path: vec![
                            // swap juno for usdc
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ujuno".to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                            },
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
                                        .to_string(),
                                },
                            },
                        ],
                        usdc_amp_whale_path: vec![
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
                                        .to_string(),
                                },
                            },
                        ],
                        usdc_bone_whale_path: vec![
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
                                        .to_string(),
                                },
                            },
                        ],

                        terraswap_multihop_router: "juno128lewlw6kv223uw4yzdffl8rnh3k9qs8vrf6kef28579w8ygccyq7m90n2"
                            .to_string(),
                        // juno_bone_whale_path: todo!(),
                    },
                    racoon_bet: RacoonBetAddresses {
                        game: "juno1h8p0jmfn06nfqpn0medn698h950vnl7v54m2azkyjdqjlzcly7jszxh7yu".to_string(),
                        juno_usdc_wynd_pair: "juno1gqy6rzary8vwnslmdavqre6jdhakcd4n2z4r803ajjmdq08r66hq7zcwrj".to_string(),
                    },
                    juno_lsds: JunoLsdAddresses {
                        bone_juno: "juno102at0mu2xeluyw9efg257yy6pyhv088qqhmp4f8wszqcwxnpdcgqsfq0nv".to_string(),
                        wy_juno: "juno18wuy5qr2mswgz7zak8yr9crhwhtur3v6mw4tcytupywxzw7sufyqgza7uh".to_string(),
                        se_juno: "juno1dlp8avgc2r6t4nnsv4yydc6lc73rjtjqvdcee9r2kf0uwuef7v0smljy8w".to_string(),
                        b_juno: "juno1dlp8avgc2r6t4nnsv4yydc6lc73rjtjqvdcee9r2kf0uwuef7v0smljy8w".to_string(),
                        amp_juno: "juno17cya4sw72h4886zsm2lk3udxaw5m8ssgpsl6nd6xl6a4ukepdgkqeuv99x".to_string(),
                    },
                },
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
                staking_denom: "ujuno".to_string(),
                take_rate_addr: "juno1takerateaddr".to_string(),
                usdc: wyndex::asset::AssetInfo::Native(
                    "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
                ),
                authzpp: AuthzppAddresses {
                    withdraw_tax: "juno1nak433pjd39et4g6jjclxk7yfmtfsd5m43su04rxe9ggttdvjwpqsumv30".to_string(),
                },
                destination_projects: DestinationProjectAddresses {
                    wynd: WyndAddresses {
                        cw20: "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9".to_string(),
                        multihop: "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w".to_string(),
                        juno_wynd_pair: "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf".to_string(),
                        wynd_usdc_pair: "juno18zk9xqj9xjm0ry39jjam8qsysj7qh49xwt4qdfp9lgtrk08sd58s2n54ve".to_string(),
                    },
                    gelotto: GelottoAddresses {
                        pick3_contract: "juno1v466lyrhsflkt9anxt4wyx7mtw8w2uyk0qxkqskqfj90rmwhph7s0dxvga".to_string(),
                        pick4_contract: "juno16xy5m05z6n4vnfzcf8cvd3anxhg4g2k8vvr4q2knv4akynfstr9qjmhdhs".to_string(),
                        pick5_contract: "juno1txn3kejj4qrehua9vlg3hk4wunqafqunfy83cz5hg2xa3z3pkgssk4tzu4".to_string(),
                    },
                    daos: DaoAddresses {
                        neta: DaoAddress {
                            cw20: "juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr".to_string(),
                            staking: "juno1a7x8aj7k38vnj9edrlymkerhrl5d4ud3makmqhx6vt3dhu0d824qh038zh".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        signal: DaoAddress {
                            cw20: "juno14lycavan8gvpjn97aapzvwmsj8kyrvf644p05r0hu79namyj3ens87650k".to_string(),
                            staking: "juno1v0km8gytmzpmtnwv8mpx26kctt5szuzudhg209fxee57yh9u2cvs88rn7p".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1p3eed298qx3nyhs3grld07jrf9vjsjsmdd2kmmh3crk87emjcx5stp409y".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        posthuman: DaoAddress {
                            cw20: "juno1rws84uz7969aaa7pej303udhlkt3j9ca0l3egpcae98jwak9quzq8szn2l".to_string(),
                            staking: "juno1jktfdt5g2d0fguvy8r8pl4gly7wps8phkwy08z6upc4nazkumrwq7lj0vn".to_string(),
                            juno_wyndex_pair: Some(
                                "juno17jv00cm4f3twr548jzayu57g9txvd4zdh54mdg9qpjs7samlphjsykylsq".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        kleomedes: DaoAddress {
                            cw20: "juno10gthz5ufgrpuk5cscve2f0hjp56wgp90psqxcrqlg4m9mcu9dh8q4864xy".to_string(),
                            staking: "juno1zqp6uh3eg09s0h24rkwukkkg3pch49g0ndc53z9l8wrvh9dhf4nsj0ur49".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1dpqgt3ja2kdxs94ltjw9ncdsexts9e3dx5qpnl20zvgdguzjelhqstf8zg".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                        cannalabs: DaoAddress {
                            cw20: "juno1vn38rzq0wc7zczp4dhy0h5y5kxh2jjzeahwe30c9cc6dw3lkyk5qn5rmfa".to_string(),
                            staking: "juno1066vq5g9qdprhgjst444rgf0zknhlqwmdnm7xyqhprt9whctzzxqdx90lu".to_string(),
                            juno_wyndex_pair: Some(
                                "juno17ckp36lmgtt7jtuggdv2j39eh4alcnl35szu6quh747nujags07swwq0nh".to_string(),
                            ),
                            wynd_wyndex_pair: Some(
                                "juno1ls5un4a8zyn4f05k0ekq5aa9uhn88y8362ww38elqfpcwllme0jqelamke".to_string(),
                            ),
                        },
                        muse: DaoAddress {
                            cw20: "juno1p8x807f6h222ur0vssqy3qk6mcpa40gw2pchquz5atl935t7kvyq894ne3".to_string(),
                            staking: "juno17gdhjxt2d5mhx6paxc85g4pr5myew8pq0lm7usdsavsfk34ldrsqqhtafc".to_string(),
                            juno_wyndex_pair: Some(
                                "juno1rcssjyqgr6vzalss77d43v30c2qpyzzg607ua8gte2shqgtvu24sg8gs8r".to_string(),
                            ),
                            wynd_wyndex_pair: None,
                        },
                    },
                    spark_ibc: SparkIbcAddresses {
                        fund: "juno1a6rna5tcl6p97rze6hnd5ug35kadqhudvr5f4mtr6s0yd5mruhss8gzrdy".to_string(),
                    },
                    balance_dao: "juno1ve7y09kvvnjk0yc2ycaq0y9thq5tct5ve6c0a5hfkt0h4jfy936qxtne5s".to_string(),
                    white_whale: WhiteWhaleSatelliteAddresses {
                        amp_whale: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF".to_string(),
                        bone_whale: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF".to_string(),
                        market: "juno1n8slcc79dmwuzdxhsesvhcncaqfg9h4czdm5t5ey8x25ajmn3xzqyde4wv".to_string(),
                        rewards: "juno184ghwgprva7dlr2hwhzgvt6mem6zx78fygk0cpw7klssmzyf67tqdtwt3h".to_string(),
                        juno_amp_whale_path: vec![
                            // swap juno for usdc
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ujuno".to_string(),
                                },

                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                            },
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
                                        .to_string(),
                                },
                            },
                        ],
                        juno_bone_whale_path: vec![
                            // swap juno for usdc
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ujuno".to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                            },
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
                                        .to_string(),
                                },
                            },
                        ],
                        usdc_amp_whale_path: vec![
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
                                        .to_string(),
                                },
                            },
                        ],
                        usdc_bone_whale_path: vec![
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C"
                                        .to_string(),
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
                                        .to_string(),
                                },
                            },
                        ],

                        terraswap_multihop_router: "juno128lewlw6kv223uw4yzdffl8rnh3k9qs8vrf6kef28579w8ygccyq7m90n2"
                            .to_string(),
                        // juno_bone_whale_path: todo!(),
                    },
                    racoon_bet: RacoonBetAddresses {
                        game: "juno1h8p0jmfn06nfqpn0medn698h950vnl7v54m2azkyjdqjlzcly7jszxh7yu".to_string(),
                        juno_usdc_wynd_pair: "juno1gqy6rzary8vwnslmdavqre6jdhakcd4n2z4r803ajjmdq08r66hq7zcwrj".to_string(),
                    },
                    juno_lsds: JunoLsdAddresses {
                        bone_juno: "juno102at0mu2xeluyw9efg257yy6pyhv088qqhmp4f8wszqcwxnpdcgqsfq0nv".to_string(),
                        wy_juno: "juno18wuy5qr2mswgz7zak8yr9crhwhtur3v6mw4tcytupywxzw7sufyqgza7uh".to_string(),
                        se_juno: "juno1dlp8avgc2r6t4nnsv4yydc6lc73rjtjqvdcee9r2kf0uwuef7v0smljy8w".to_string(),
                        b_juno: "juno1dlp8avgc2r6t4nnsv4yydc6lc73rjtjqvdcee9r2kf0uwuef7v0smljy8w".to_string(),
                        amp_juno: "juno17cya4sw72h4886zsm2lk3udxaw5m8ssgpsl6nd6xl6a4ukepdgkqeuv99x".to_string(),
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
