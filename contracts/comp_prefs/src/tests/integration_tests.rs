use crate::{
    interface::YmosCompPrefsContract,
    msg::{CompPrefStatus, ExecuteMsgFns, InstantiateMsg, QueryMsgFns},
    state::{
        CompPref, CompoundingFrequency, EndType, InactiveStatus, StoreSettings,
        UnverifiedUserCompPref, UserCompPref,
    },
    ContractError,
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Decimal, StdError, Timestamp, Uint64};
use cw_orch::{anyhow, prelude::*};

#[cw_serde]
pub struct ExampleCompoundPrefs {
    pub user_address: String,
    pub tax_fee: Option<Decimal>,
}

#[test]
pub fn test() -> anyhow::Result<()> {
    let admin = Addr::unchecked("tprl1adminf5846rxzp3fwlswy08fz8ccuwk03k57y");
    let outpost_contract = Addr::unchecked("tprl1outpost15846rxzp3fwlswy08fz8ccuwk03k57y");
    let user = Addr::unchecked("tprl1user15846rxzp3fwlswy08fz8ccuwk03k57y");
    let user_2 = Addr::unchecked("tprl1user25846rxzp3fwlswy08fz8ccuwk03k57y");
    let user_3 = Addr::unchecked("tprl1user35846rxzp3fwlswy08fz8ccuwk03k57y");

    let mock = Mock::new(&admin);

    let comp_prefs_contract = YmosCompPrefsContract::new("mock:comp_pref_contract", mock.clone());

    comp_prefs_contract.upload()?;

    comp_prefs_contract.instantiate(
        &InstantiateMsg {
            admin: None,
            chain_id: "temporal-1".to_string(),
            days_to_prune: 180,
        },
        Some(&admin),
        None,
    )?;

    let prefs: UnverifiedUserCompPref = UnverifiedUserCompPref {
        outpost_address: outpost_contract.to_string(),
        address: user.to_string(),
        strat_id: Uint64::from(1u64),
        strategy_settings: to_json_binary(&ExampleCompoundPrefs {
            user_address: user.to_string(),
            tax_fee: Some(Decimal::percent(10)),
        })?,
        comp_period: CompoundingFrequency::Daily,
        pub_key: "123".to_string(),
        expires: Timestamp::from_nanos(1671797519879305533),
    };
    let verified_prefs: CompPref = CompPref {
        chain_id: "temporal-1".to_string(),
        user_comp_pref: UserCompPref {
            outpost_address: outpost_contract.clone(),
            strat_id: 1u64,
            strategy_settings: to_json_binary(&ExampleCompoundPrefs {
                user_address: user.to_string(),
                tax_fee: Some(Decimal::percent(10)),
            })?,
            address: user.clone(),
            comp_period: prefs.comp_period.clone(),
            pub_key: prefs.pub_key.clone(),
            expires: prefs.expires,
        },
        is_inactive: None,
        created_at: Timestamp::from_nanos(1571797419879305533),
        updated_at: Timestamp::from_nanos(1571797419879305533),
    };

    let prefs_2: UnverifiedUserCompPref = UnverifiedUserCompPref {
        outpost_address: outpost_contract.to_string(),
        address: user_2.to_string(),
        strat_id: Uint64::from(1u64),
        strategy_settings: to_json_binary(&ExampleCompoundPrefs {
            user_address: user_2.to_string(),
            tax_fee: Some(Decimal::percent(5)),
        })?,
        comp_period: CompoundingFrequency::Daily,
        pub_key: "234".to_string(),
        expires: Timestamp::from_nanos(1671797519879305533),
    };
    let verified_prefs_2: CompPref = CompPref {
        chain_id: "temporal-1".to_string(),
        user_comp_pref: UserCompPref {
            outpost_address: outpost_contract.clone(),
            strat_id: 1u64,
            strategy_settings: to_json_binary(&ExampleCompoundPrefs {
                user_address: user_2.to_string(),
                tax_fee: Some(Decimal::percent(5)),
            })?,
            address: user_2.clone(),
            comp_period: prefs_2.comp_period.clone(),
            pub_key: prefs_2.pub_key.clone(),
            expires: prefs_2.expires,
        },
        is_inactive: None,
        created_at: Timestamp::from_nanos(1571797419879305533),
        updated_at: Timestamp::from_nanos(1571797419879305533),
    };

    let prefs_3: UnverifiedUserCompPref = UnverifiedUserCompPref {
        outpost_address: outpost_contract.to_string(),
        address: user_3.to_string(),
        strat_id: Uint64::from(1u64),
        strategy_settings: to_json_binary(&ExampleCompoundPrefs {
            user_address: user_3.to_string(),
            tax_fee: Some(Decimal::percent(5)),
        })?,
        comp_period: CompoundingFrequency::Daily,
        pub_key: "345".to_string(),
        expires: Timestamp::from_nanos(1671797524879305533),
    };
    let verified_prefs_3: CompPref = CompPref {
        chain_id: "temporal-1".to_string(),
        user_comp_pref: UserCompPref {
            outpost_address: outpost_contract.clone(),
            strat_id: 1u64,
            strategy_settings: to_json_binary(&ExampleCompoundPrefs {
                user_address: user_3.to_string(),
                tax_fee: Some(Decimal::percent(5)),
            })?,
            address: user_3.clone(),
            comp_period: prefs_3.comp_period.clone(),
            pub_key: prefs_3.pub_key.clone(),
            expires: prefs_3.expires,
        },
        is_inactive: Some(InactiveStatus {
            end_type: EndType::Cancellation,
            ended_at: Timestamp::from_nanos(1571797424879305533),
        }),
        created_at: Timestamp::from_nanos(1571797419879305533),
        updated_at: Timestamp::from_nanos(1571797419879305533),
    };

    // only the admin can change the admin
    comp_prefs_contract
        .call_as(&user)
        .set_admin(user.to_string())
        .unwrap_err();

    assert_eq!(
        comp_prefs_contract.store_settings()?,
        StoreSettings {
            // admin should stay unchanged
            admin: admin.clone(),
            chain_id: "temporal-1".to_string(),
            days_to_prune: 180,
        },
        "store settings should be set correctly"
    );

    assert_eq!(
        comp_prefs_contract.allowed_strategy_ids()?,
        vec![] as Vec<Uint64>,
        "allowed strategy ids should be empty"
    );

    assert!(
        comp_prefs_contract
            .set_compounding_preferences(prefs.clone())
            .is_err(),
        "should error on invalid strat id since it has not been added"
    );

    // non-admin cannot add allowed strategy ids
    comp_prefs_contract
        .call_as(&user)
        .add_allowed_strategy_id(Uint64::from(2u64))
        .unwrap_err();

    // admin should be able to add allowed strategy ids
    comp_prefs_contract
        .call_as(&admin)
        .add_allowed_strategy_id(Uint64::from(1u64))?;

    assert_eq!(
        comp_prefs_contract.allowed_strategy_ids()?,
        vec![Uint64::from(1u64)],
        "only strat 1 should be allowed"
    );

    comp_prefs_contract
        .call_as(&user)
        .set_compounding_preferences(prefs.clone())?;

    assert_eq!(
        comp_prefs_contract
            .strategy_preferences_by_user_and_strat_id(Uint64::from(1u64), user.to_string())?,
        Some(verified_prefs.clone()),
        "should be able to get the compounding preferences for the user"
    );

    // failing prefs storage
    comp_prefs_contract
        .call_as(&user)
        .set_compounding_preferences(prefs_2.clone())
        .expect_err("Shouldn't be able to set compounding preferences for another user");

    // store user2's prefs
    comp_prefs_contract
        .call_as(&user_2)
        .set_compounding_preferences(prefs_2.clone())?;

    comp_prefs_contract
        .call_as(&user_3)
        .set_compounding_preferences(prefs_3.clone())?;

    mock.clone().next_block()?;

    comp_prefs_contract
        .call_as(&user_3)
        .cancel_compounding_preferences(Uint64::from(1u64))?;

    assert_eq!(
        comp_prefs_contract.strategy_preferences_by_strat_id(
            Uint64::from(1u64),
            None,
            None,
            None
        )?,
        vec![
            verified_prefs.clone(),
            verified_prefs_2.clone(),
            verified_prefs_3.clone()
        ],
        "should be able to get the compounding preferences for the user"
    );
    assert_eq!(
        comp_prefs_contract.strategy_preferences_by_strat_id(
            Uint64::from(1u64),
            Some(2u16),
            None,
            None
        )?,
        vec![
            verified_prefs.clone(),
            verified_prefs_2.clone(),
            // verified_prefs_3.clone()
        ],
        "should be able to limit the number of results returned"
    );

    assert_eq!(
        comp_prefs_contract.strategy_preferences_by_strat_id(
            Uint64::from(1u64),
            None,
            None,
            Some(CompPrefStatus::Active),
        )?,
        vec![verified_prefs.clone(), verified_prefs_2.clone()],
        "should be able to filter out inactive compounding preferences"
    );

    assert_eq!(
        comp_prefs_contract.strategy_preferences_by_strat_id(
            Uint64::from(1u64),
            Some(2u16),
            None,
            Some(CompPrefStatus::Inactive),
        )?,
        vec![verified_prefs_3.clone()],
        "should be able to filter out active compounding preferences"
    );

    Ok(())
}
