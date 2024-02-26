use cosmwasm_std::Uint64;
use cw_orch::{
    anyhow,
    daemon::{
        networks::{migaloo::MIGALOO_NETWORK},
        ChainInfo, ChainKind, DaemonBuilder,
    },
    prelude::*,
};

use migaloo_destinations::comp_prefs::{
    DaoAddress, DaoDaoAddresses, Denoms, DestProjectSwapRoutes, GinkouAddresses,
    MigalooProjectAddresses, SatelliteMarketAddresses, UsdcRoutes, VaultAddresses,
    WhaleLsdAddresses, WhaleRoutes,
};
use tokio::runtime::Runtime;
use white_whale::pool_network::{asset::AssetInfo, router::SwapOperation};

use ymos_comp_prefs::{
    msg::{ExecuteMsgFns as CompPrefExecuteMsgFns},
    YmosCompPrefsContract,
};
use ymos_migaloodca_outpost::{
    msg::ExecuteMsgFns as MigaloodcaExecuteMsgFns,
};
use ymos_migaloostake_outpost::{
    msg::ExecuteMsgFns as MigaloostakeExecuteMsgFns,
};

const YMOS_CONDUCTOR: &str = "migaloo1f49xq0rmah39sk58aaxq6gnqcvupee7jne0wep";
const YMOS_FEE_SHARE_COLLECTOR: &str = "migaloo1ewdttrv2ph7762egx4n2309h3m9r4z9pxsg48n";

pub fn main() -> anyhow::Result<()> {
    let denoms: Denoms = Denoms {
        usdc: "ibc/80E8F826480B995AE28C1EE86106C1BE2034FF1966579D29951CF61885458040".to_string(),
        whale: "uwhale".to_string(),
        bwhale:
            "factory/migaloo1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqdhts4u/boneWhale"
                .to_string(),
        ampwhale:
            "factory/migaloo1436kxs0w2es6xlqpp9rd35e3d0cjnw4sv8j3a7483sgks29jqwgshqdky4/ampWHALE"
                .to_string(),
        arbwhale:
            "factory/migaloo1ey4sn2mkmhew4pdrzk90l9acluvas25qlhuvsfgssw42ugz8yjlqx92j9l/arbWHALE"
                .to_string(),
        ash: "factory/migaloo1erul6xyq0gk6ws98ncj7lnq9l4jn4gnnu9we73gdz78yyl2lr7qqrvcgup/ash"
            .to_string(),
        guppy: "factory/migaloo1etlu2h30tjvv8rfa4fwdc43c92f6ul5w9acxzk/uguppy".to_string(),
        rac: "factory/migaloo1eqntnl6tzcj9h86psg4y4h6hh05g2h9nj8e09l/urac".to_string(),
        musdc: "migaloo10nucfm2zqgzqmy7y7ls398t58pjt9cwjsvpy88y2nvamtl34rgmqt5em2v".to_string(),
        bluna: "ibc/40C29143BF4153B365089E40E437B7AA819672646C45BB0A5F1E10915A0B6708".to_string(),
        ampluna: "ibc/05238E98A143496C8AF2B6067BABC84503909ECE9E45FBCBAC2CBA5C889FD82A".to_string(),
    };
    let migaloostake_project_addresses = ymos_migaloostake_outpost::msg::ContractAddresses {
        staking_denom: "uwhale".to_string(),
        // needs to be switchout for mainnet
        take_rate_addr: YMOS_FEE_SHARE_COLLECTOR.to_string(),
        usdc: AssetInfo::NativeToken {
            denom: "ibc/80E8F826480B995AE28C1EE86106C1BE2034FF1966579D29951CF61885458040"
                .to_string(),
        },
        authzpp: ymos_migaloostake_outpost::msg::AuthzppAddresses {
            withdraw_tax: YMOS_FEE_SHARE_COLLECTOR.to_string(),
        },
        destination_projects:
            migaloo_destinations::comp_prefs::MigalooDestinationProjectAddresses {
                denoms: denoms.clone(),
                swap_routes: DestProjectSwapRoutes {
                    whale_usdc_pool:
                        "migaloo1nha6qlam4p92cf9j64qv9he40xyf3akl2m9w5dukftmf2ryrxz5qy650zh"
                            .to_string(),
                    whale_bwhale_pool:
                        "migaloo1dg5jrt89nddtymjx5pzrvdvdt0m4zl3l2l3ytunl6a0kqd7k8hss594wy6"
                            .to_string(),
                    whale_ampwhale_pool:
                        "migaloo1ull9s4el2pmkdevdgrjt6pwa4e5xhkda40w84kghftnlxg4h3knqpm5u3n"
                            .to_string(),
                    whale_ash_pool:
                        "migaloo1u4npx7xvprwanpru7utv8haq99rtfmdzzw6p3hpfc38n7zmzm42q8ydga3"
                            .to_string(),

                    whale_rac_pool:
                        "migaloo1crsvm4qddplxhag29nd2zyw6k6jzh06hlcctya4ynfvuhhu3yt4q0pn4t3"
                            .to_string(),
                    whale: WhaleRoutes {},
                    usdc: UsdcRoutes {
                        whale: vec![SwapOperation::TerraSwap {
                            offer_asset_info: AssetInfo::NativeToken { denom: denoms.usdc },
                            ask_asset_info: AssetInfo::NativeToken {
                                denom: denoms.whale,
                            },
                        }],
                        // guppy: "".to_string(),
                    },
                },
                projects: MigalooProjectAddresses {
                    terraswap_multihop_router:
                        "migaloo1tma28exp38q92c69r8uujhphxy95xa4awq2cudqqg3nhzkhnrg5s4r60en"
                            .to_string(),
                    daodao: DaoDaoAddresses {
                        racoon_supply_dao: DaoAddress {
                            denom: denoms.rac,
                            staking_address:
                                "migaloo1398xz7e3zrv9ryx79lu8z46l3rqzy7tgta7dvhfx6853g8mp7fls43t7yf"
                                    .to_string(),
                        },
                        guppy_dao: DaoAddress {
                            denom: denoms.guppy,
                            staking_address:
                                "migaloo1w3n0kcmrtwnj8dj6t6p7gm9szv0nsamezqw58374zz06jvstvr9q539fjw"
                                    .to_string(),
                        },
                    },
                    spark_ibc: "migaloo1gxw00ht2jz490lkg46a884l7knz7sspk0djffsmlcnsf8a0cdnksnj2y0j"
                        .to_string(),
                    white_whale_satellite: SatelliteMarketAddresses {
                        market:
                            "migaloo1692nylpkryu7q00eukt93egtqu657z33nf0tedp0ps6htm8aty6qjdlpvh"
                                .to_string(),
                        rewards:
                            "migaloo13e3ywqudfz92pq2sxwuwf35rdtp3hvz7xu0danyk38dchevywraswfgnqx"
                                .to_string(),
                    },
                    racoon_bet:
                        "migaloo1vjt5hsgneptmdtxyadm440qdf6r5y56fmurf3k870qrvjk4pfgxqapwlsm"
                            .to_string(),
                    whale_lsd: WhaleLsdAddresses {
                        bone_whale:
                            "migaloo1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqdhts4u"
                                .to_string(),
                        amp_whale:
                            "migaloo1436kxs0w2es6xlqpp9rd35e3d0cjnw4sv8j3a7483sgks29jqwgshqdky4"
                                .to_string(),
                    },
                    furnace: "migaloo1erul6xyq0gk6ws98ncj7lnq9l4jn4gnnu9we73gdz78yyl2lr7qqrvcgup"
                        .to_string(),
                    vaults: VaultAddresses {
                        amp_usdc:
                            "migaloo12ye2j33d6lv84x8zq6dpjj2hepzn2njnrnnwlmuam0v0eczr787qhmf7en"
                                .to_string(),
                        arb_whale:
                            "migaloo1ey4sn2mkmhew4pdrzk90l9acluvas25qlhuvsfgssw42ugz8yjlqx92j9l"
                                .to_string(),
                        amp_ash:
                            "migaloo1cmcnld5q4z9nltml664nuxthcrz5r9vpfv0efgadxj4pwl3ry8yq26nk76"
                                .to_string(),
                    },
                    ginkou: GinkouAddresses {
                        deposit:
                            "migaloo1qelh4gv5drg3yhj282l6n84a6wrrz033kwyak3ee3syvqg3mu3msgphpk4"
                                .to_string(),
                        borrow:
                            "migaloo10nucfm2zqgzqmy7y7ls398t58pjt9cwjsvpy88y2nvamtl34rgmqt5em2v"
                                .to_string(),
                    },
                    ecosystem_stake:
                        "migaloo190qz7q5fu4079svf890h4h3f8u46ty6cxnlt78eh486k9qm995hquuv9kd"
                            .to_string(),
                },
            },
    };
    let migaloo_dca_project_addresses = ymos_migaloodca_outpost::msg::ContractAddresses {
        staking_denom: "uwhale".to_string(),
        take_rate_addr: YMOS_FEE_SHARE_COLLECTOR.to_string(),
        usdc: migaloostake_project_addresses.usdc.clone(),
        authzpp: ymos_migaloodca_outpost::msg::AuthzppAddresses {},
        destination_projects: migaloostake_project_addresses.destination_projects.clone(),
    };

    let rt = Runtime::new().unwrap();
    dotenv::dotenv().ok();
    env_logger::init();

    let migaloo: ChainInfo = ChainInfo {
        kind: ChainKind::Mainnet,
        chain_id: "migaloo-1",
        gas_denom: "uwhale",
        gas_price: 1f64,
        grpc_urls: &[
            "https://migaloo-grpc.lavenderfive.com:443",
            "http://migaloo.grpc.kjnodes.com:14990",
            "http://grpc-migaloo.cosmos-spaces.cloud:2290",
            "http://migaloo-grpc.polkachu.com:20790",
            "http://migaloo-grpc.cosmosrescue.com:9090",
        ],
        network_info: MIGALOO_NETWORK,
        lcd_url: None,
        fcd_url: None,
    };

    let migaloo_chain = DaemonBuilder::default()
        .handle(rt.handle())
        .chain(migaloo.clone())
        .build()?;

    let migaloo_comp_prefs = YmosCompPrefsContract::new(
        "Yieldmos Juno Compounding Preferences",
        migaloo_chain.clone(),
    );

    let _migaloostake = ymos_migaloostake_outpost::YmosMigaloostakeOutpost::new(
        "Yieldmos Migaloostake Outpost",
        migaloo_chain.clone(),
    );

    let migaloodca = ymos_migaloodca_outpost::YmosMigaloodcaOutpost::new(
        "Yieldmos Migaloo DCA Outpost",
        migaloo_chain.clone(),
    );

    migaloo_comp_prefs.upload_if_needed()?;
    // migaloostake.upload_if_needed()?;
    migaloodca.upload_if_needed()?;

    // migaloo_comp_prefs contract upload
    if migaloo_comp_prefs.address().is_err() {
        migaloo_comp_prefs.instantiate(
            &ymos_comp_prefs::msg::InstantiateMsg {
                admin: None,
                chain_id: "juno-1".to_string(),
                days_to_prune: 180u16,
            },
            Some(&Addr::unchecked(migaloo_chain.sender().to_string())),
            None,
        )?;

        // dca
        migaloo_comp_prefs.add_allowed_strategy_id(Uint64::from(80100u64))?;
        // whale staking
        migaloo_comp_prefs.add_allowed_strategy_id(Uint64::from(80101u64))?;
        // sat market
        migaloo_comp_prefs.add_allowed_strategy_id(Uint64::from(80102u64))?;
        // // white whale sat market
        // migaloo_comp_prefs
        //     .add_allowed_strategy_id(Uint64::from(80103u64))?;
    } else {
        migaloo_comp_prefs.migrate(
            &ymos_comp_prefs::msg::MigrateMsg {},
            migaloo_comp_prefs.code_id()?,
        )?;
    }
    println!("migaloo_comp_prefs: {}", migaloo_comp_prefs.addr_str()?);

    // // migaloostake contract upload
    // if migaloostake.address().is_err() {
    //     migaloostake.instantiate(
    //         &ymos_migaloostake_outpost::msg::InstantiateMsg {
    //             admin: Some(migaloo_chain.sender().to_string()),
    //             project_addresses: migaloostake_project_addresses.clone(),
    //         },
    //         Some(&Addr::unchecked(migaloo_chain.sender().to_string())),
    //         None,
    //     )?;

    //     // add yieldmos.juno as an authorized compounder
    //     migaloostake
    //         .add_authorized_compounder(YMOS_CONDUCTOR.to_string())
    //         .unwrap();
    // } else {
    //     migaloostake.migrate(
    //         &ymos_migaloostake_outpost::msg::MigrateMsg {
    //             project_addresses: Some(migaloostake_project_addresses.clone()),
    //         },
    //         migaloostake.code_id()?,
    //     )?;
    // }
    // println!("migaloostake: {}", migaloostake.addr_str()?);

    // migaloodca contract upload
    if migaloodca.address().is_err() {
        migaloodca.instantiate(
            &ymos_migaloodca_outpost::msg::InstantiateMsg {
                admin: Some(migaloo_chain.sender().to_string()),
                project_addresses: migaloo_dca_project_addresses.clone(),
            },
            Some(&Addr::unchecked(migaloo_chain.sender().to_string())),
            None,
        )?;

        // add yieldmos.juno as an authorized compounder
        migaloodca
            .add_authorized_compounder(YMOS_CONDUCTOR.to_string())
            .unwrap();
    } else {
        migaloodca.migrate(
            &ymos_migaloodca_outpost::msg::MigrateMsg {
                project_addresses: Some(migaloo_dca_project_addresses.clone()),
            },
            migaloodca.code_id()?,
        )?;
    }
    println!("migaloodca: {}", migaloodca.addr_str()?);

    Ok(())
}
