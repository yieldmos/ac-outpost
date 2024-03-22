use crate::{
    contract::{execute, instantiate, query},
    msg::{AuthzppAddresses, ContractAddresses, InstantiateMsg},
    tests::{multitest::OutpostContract, staking::Staking},
};
use cosmwasm_std::{coin, coins, testing::mock_env, Addr, CosmosMsg, Decimal, Delegation, Empty, Validator};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, StakingInfo};
use cw_orch::osmosis_test_tube::{
    osmosis_test_tube::{Gamm, Module},
    OsmosisTestTube,
};
use cw_orch::prelude::*;
use osmosis_destinations::{
    comp_prefs::{
        DaoDaoAddresses, DestProjectSwapRoutes, MembraneAddresses, OsmosisDestinationProjectAddresses,
        OsmosisProjectAddresses, RedbankAddresses,
    },
    pools::{Denoms, OsmoPools, OsmosisKnownPoolListing, UsdcPools},
};
use outpost_utils::msg_gen::{create_generic_grant_msg, GenericAuthorizationType};
use ymos_osmostake_outpost::YmosOsmosisstakeOutpost;

#[test]
fn instantiate_with_defaults() {
    let mut chain = OsmosisTestTube::new(coins(1_000_000_000_000, "uosmo"));

    let admin = chain.init_account(coins(100_000_000_000, "uosmo")).unwrap();
    // let user = chain.init_account(coins(1_000_000_000, "uosmo")).unwrap();
    // let treasury = chain.init_account(coins(0, "uosmo")).unwrap();

    let osmostake_contract = YmosOsmosisstakeOutpost::new("osmostake_contract", chain.clone());

    let upload_resp = osmostake_contract.upload();
    assert!(upload_resp.is_ok(), "Upload osmostake failed");

    // let withdraw_tax_contract = withdraw_rewards_tax_grant::WithdrawRewardsTaxGrant::new("withdraw_tax_contract", chain);

    // withdraw_tax_contract
    //     .call_as(&admin)
    //     .upload("../../vendor_wasm/withdraw_rewards_tax_grant.wasm")
    //     .unwrap();

    // withdraw_tax_contract.call_as(&admin).instantiate().unwrap();

    // let existing_validators = staking
    //     .query_validators(&QueryValidatorsRequest {
    //         status: "".to_string(),
    //         pagination: None,
    //     })
    //     .unwrap();

    // // set aside the first validator for testing
    // let validator = existing_validators.validators.first().unwrap().clone();

    // let sender = Addr::unchecked("sender");

    // let mut app = App::new(|_router, _api, _storage| {
    //     // router.bank
    //     // .init_balance(storage, &sender, coins(100_000, "ubtc"))
    //     // .unwrap();
    // });

    // let contract_id = app.store_code(auctioning_contract());

    // let _contract = OutpostContract::instantiate(
    //     &mut app,
    //     contract_id,
    //     &sender,
    //     None,
    //     "Test Outpost",
    //     // &coins(100_000, "ubtc"),
    //     &InstantiateMsg {
    //         admin: None,
    //         project_addresses: ContractAddresses {
    //             staking_denom: "uosmo".to_string(),
    //             authzpp: AuthzppAddresses {
    //                 withdraw_tax: "withdraw_tax_contract".to_string(),
    //             },
    //             destination_projects: OsmosisDestinationProjectAddresses {
    //                 denoms: Denoms::default(),
    //                 swap_routes: DestProjectSwapRoutes::default(),
    //                 projects: OsmosisProjectAddresses {
    //                     daodao: DaoDaoAddresses {},
    //                     redbank: RedbankAddresses {
    //                         credit_manager: "redbank_credit_manager".to_string(),
    //                     },
    //                     ion_dao: "ion_dao".to_string(),
    //                     milky_way_bonding: "milky_way_bonding".to_string(),
    //                     eris_amposmo_bonding: "eris_amposmo_bonding".to_string(),
    //                     membrane: MembraneAddresses {
    //                         cdp: "membrane_cdp".to_string(),
    //                         staking: "mbrn_staking".to_string(),
    //                     },
    //                 },
    //             },
    //         },
    //         max_tax_fee: Decimal::percent(7),
    //         take_rate_address: "takerate_addr".to_string(),
    //     },
    // )
    // .unwrap();

    // assert_eq!(app.wrap().query_all_balances(&sender).unwrap(), &[])
}

#[test]
fn validator_only_compounding() {
    // let contract_admin_addr = Addr::unchecked("admin_wallet");
    // let delegator_addr = Addr::unchecked("delegator_wallet");
    // let start_validator_addr = Addr::unchecked("validator_1");
    // let end_validator_addr = Addr::unchecked("validator_2");

    // let mut app = App::new(|router, api, storage| {
    //     router
    //         .bank
    //         .init_balance(storage, &delegator_addr, coins(100_000, "ubtc"))
    //         .unwrap();

    //     router
    //         .staking
    //         .setup(
    //             storage,
    //             StakingInfo {
    //                 bonded_denom: "ubtc".to_string(),
    //                 unbonding_time: 10u64,
    //                 apr: Decimal::percent(1000),
    //             },
    //         )
    //         .unwrap();

    //     router
    //         .staking
    //         .add_validator(
    //             api,
    //             storage,
    //             &mock_env().block,
    //             Validator {
    //                 address: start_validator_addr.to_string(),
    //                 commission: Decimal::one(),
    //                 max_commission: Decimal::one(),
    //                 max_change_rate: Decimal::one(),
    //             },
    //         )
    //         .unwrap();

    //     router
    //         .staking
    //         .add_validator(
    //             api,
    //             storage,
    //             &mock_env().block,
    //             Validator {
    //                 address: end_validator_addr.to_string(),
    //                 commission: Decimal::one(),
    //                 max_commission: Decimal::one(),
    //                 max_change_rate: Decimal::one(),
    //             },
    //         )
    //         .unwrap();
    // });

    // let contract_id = app.store_code(auctioning_contract());

    // let contract = OutpostContract::instantiate(
    //     &mut app,
    //     contract_id,
    //     &contract_admin_addr,
    //     None,
    //     "Test Outpost",
    //     &InstantiateMsg {
    //         admin: None,
    //         project_addresses: ContractAddresses {
    //             staking_denom: "uosmo".to_string(),
    //             authzpp: AuthzppAddresses {
    //                 withdraw_tax: "withdraw_tax_contract".to_string(),
    //             },
    //             destination_projects: OsmosisDestinationProjectAddresses {
    //                 denoms: Denoms::default(),
    //                 swap_routes: DestProjectSwapRoutes::default(),
    //                 projects: OsmosisProjectAddresses {
    //                     daodao: DaoDaoAddresses {},
    //                     redbank: RedbankAddresses {
    //                         credit_manager: "redbank_credit_manager".to_string(),
    //                     },
    //                     ion_dao: "ion_dao".to_string(),
    //                     milky_way_bonding: "milky_way_bonding".to_string(),
    //                     eris_amposmo_bonding: "eris_amposmo_bonding".to_string(),
    //                     membrane: MembraneAddresses {
    //                         cdp: "membrane_cdp".to_string(),
    //                         staking: "mbrn_staking".to_string(),
    //                     },
    //                 },
    //             },
    //         },
    //         max_tax_fee: Decimal::percent(7),
    //         take_rate_address: "takerate_addr".to_string(),
    //     },
    // )
    // .unwrap();

    // app.execute_multi(
    //     delegator_addr.clone(),
    //     vec![
    //         CosmosMsg::Staking(cosmwasm_std::StakingMsg::Delegate {
    //             validator: start_validator_addr.to_string(),
    //             amount: coin(100_000, "ubtc"),
    //         }),
    //         // create_generic_grant_msg(
    //         //     delegator_addr.to_string(),
    //         //     &contract.addr(),
    //         //     GenericAuthorizationType::Delegation,
    //         // ),
    //         // create_generic_grant_msg(
    //         //     delegator_addr.to_string(),
    //         //     &contract.addr(),
    //         //     GenericAuthorizationType::WithdrawDelegatorRewards,
    //         // ),
    //     ],
    // )
    // .unwrap();

    // assert_eq!(app.wrap().query_all_balances(&delegator_addr).unwrap(), &[]);
    // assert_eq!(
    //     app.wrap().query_all_delegations(delegator_addr.clone()).unwrap(),
    //     vec![Delegation {
    //         delegator: delegator_addr.clone(),
    //         validator: start_validator_addr.to_string(),
    //         amount: coin(100_000, "ubtc".to_string())
    //     }]
    // );

    // app.update_block(next_block);

    // // let err = contract
    // //     .compound_funds(
    // //         &mut app,
    // //         &delegator_addr.clone(),
    // //         CompoundPrefs {
    // //             relative: vec![DestinationAction {
    // //                 destination: DestinationProject::JunoStaking {
    // //                     validator_address: end_validator_addr.to_string(),
    // //                 },
    // //                 amount: RelativeQty {
    // //                     quantity: 1_000_000_000_000_000_000u128.into(),
    // //                 },
    // //             }],
    // //         },
    // //         delegator_addr.to_string(),
    // //     )
    // //     .unwrap_err();

    // // println!("{}", err)
}
