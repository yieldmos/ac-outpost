use cosmwasm_std::{to_json_binary, Addr, Decimal, Timestamp};

use crate::{
    msg::CompPrefStatus,
    state::{CompPref, CompoundingFrequency, UserCompPref},
};

use super::integration_tests::ExampleCompoundPrefs;

#[test]
fn test_comp_pref_status_matching() {
    let user_address = Addr::unchecked("tprl1user15846rxzp3fwlswy08fz8ccuwk03k57y");
    let pref = CompPref {
        user_comp_pref: UserCompPref {
            address: user_address.clone(),
            outpost_address: Addr::unchecked("tprl1outpost15846rxzp3fwlswy08fz8ccuwk03k57y"),
            strat_id: 1u64,
            strategy_settings: to_json_binary(&ExampleCompoundPrefs {
                user_address: user_address.to_string(),
                tax_fee: Some(Decimal::percent(5)),
            })
            .unwrap(),
            comp_period: CompoundingFrequency::Daily,
            pub_key: "1".to_string(),
            expires: Timestamp::from_seconds(2),
        },
        is_inactive: None,
        chain_id: "temporal-1".to_string(),
        created_at: Timestamp::from_seconds(1),
        updated_at: Timestamp::from_seconds(1),
    };

    assert!(pref.is_active(&Timestamp::from_seconds(2)));
    assert_eq!(pref.ended_timestamp(&Timestamp::from_seconds(2)), None);
    assert_eq!(pref.cancelled_timestamp(), None);
    assert_eq!(pref.expired_timestamp(&Timestamp::from_seconds(2)), None);

    assert!(!pref.is_active(&Timestamp::from_seconds(3)));
    assert_eq!(
        pref.ended_timestamp(&Timestamp::from_seconds(3)),
        Some(Timestamp::from_seconds(2))
    );
    assert_eq!(
        pref.expired_timestamp(&Timestamp::from_seconds(3)),
        Some(Timestamp::from_seconds(2))
    );

    assert!(pref.matches_status_filter(&None, &Timestamp::from_seconds(2)));
    assert!(pref.matches_status_filter(&Some(CompPrefStatus::Active), &Timestamp::from_seconds(2)));
    assert!(!pref.matches_status_filter(&Some(CompPrefStatus::Active), &Timestamp::from_seconds(3)));
    assert!(
        pref.matches_status_filter(&Some(CompPrefStatus::Inactive), &Timestamp::from_seconds(3))
    );
    assert!(
        !pref.matches_status_filter(&Some(CompPrefStatus::Inactive), &Timestamp::from_seconds(2))
    );
    assert!(!pref.matches_status_filter(
        &Some(CompPrefStatus::Cancelled),
        &Timestamp::from_seconds(3)
    ));
}
