use anybuf::Anybuf;
use cosmwasm_std::Uint64;
use cw_orch::{anyhow, daemon::{DaemonBuilder, ChainInfo, ChainKind, networks::juno::JUNO_NETWORK}, prelude::*};

use juno_destinations::comp_prefs::DaoAddress;
use tokio::runtime::Runtime;
use ymos_junodca_outpost::msg::ExecuteMsgFns as JunodcaExecuteMsgFns;
use ymos_junostake_outpost::msg::ExecuteMsgFns as JunostakeExecuteMsgFns;
use ymos_wyndstake_outpost::msg::ExecuteMsgFns as WyndstakeExecuteMsgFns;
use ymos_junowwmarket_outpost::msg::{ExecuteMsgFns as JunowwmarketExecuteMsgFns, TerraswapRouteAddresses};
use white_whale::pool_network::{asset::AssetInfo as WWAssetInfo, router::SwapOperation};
use ymos_comp_prefs::{msg::{ExecuteMsgFns as CompPrefExecuteMsgFns}, YmosCompPrefsContract};

#[derive(PartialEq, Eq, Debug)]
pub enum DeploymentType {
    Prod,
    Dev,
}

const YMOS_CONDUCTOR: &str = "juno1f49xq0rmah39sk58aaxq6gnqcvupee7jgl90tn";
const YMOS_FEE_SHARE_COLLECTOR: &str = "juno1ewdttrv2ph7762egx4n2309h3m9r4z9pakz54p";

// struct DaoAddress {
//     cw20: String,
//     staking: String,
//     juno_wyndex_pair: Option<String>,
//     wynd_wyndex_pair: Option<String>
// }

pub fn main() -> anyhow::Result<()> {
    let junostake_project_addresses = ymos_junostake_outpost::msg::ContractAddresses {
        staking_denom: "ujuno".to_string(),
        // needs to be switchout for mainnet
        take_rate_addr: YMOS_FEE_SHARE_COLLECTOR.to_string(),
        usdc: wyndex::asset::AssetInfo::Native(
            "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
        ),
        authzpp: ymos_junostake_outpost::msg::AuthzppAddresses {
            withdraw_tax: "juno1nak433pjd39et4g6jjclxk7yfmtfsd5m43su04rxe9ggttdvjwpqsumv30"
                .to_string(),
        },
        destination_projects: juno_destinations::comp_prefs::DestinationProjectAddresses {
            wynd: juno_destinations::comp_prefs::WyndAddresses {
                cw20: "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9".to_string(),
                multihop: "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w"
                    .to_string(),
                juno_wynd_pair: "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf"
                    .to_string(),
                wynd_usdc_pair: "juno18zk9xqj9xjm0ry39jjam8qsysj7qh49xwt4qdfp9lgtrk08sd58s2n54ve".to_string()
            },
            gelotto: juno_destinations::comp_prefs::GelottoAddresses {
                pick3_contract: "juno1v466lyrhsflkt9anxt4wyx7mtw8w2uyk0qxkqskqfj90rmwhph7s0dxvga"
                    .to_string(),
                pick4_contract: "juno16xy5m05z6n4vnfzcf8cvd3anxhg4g2k8vvr4q2knv4akynfstr9qjmhdhs"
                    .to_string(),
                pick5_contract: "juno1txn3kejj4qrehua9vlg3hk4wunqafqunfy83cz5hg2xa3z3pkgssk4tzu4"
                    .to_string(),
            },
            daos: juno_destinations::comp_prefs::DaoAddresses {
                neta: DaoAddress {
                    cw20: "juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr"
                        .to_string(),
                    staking: "juno1a7x8aj7k38vnj9edrlymkerhrl5d4ud3makmqhx6vt3dhu0d824qh038zh"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno1h6x5jlvn6jhpnu63ufe4sgv4utyk8hsfl5rqnrpg2cvp6ccuq4lqwqnzra"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: None,
                },
                signal: DaoAddress {
                    cw20: "juno14lycavan8gvpjn97aapzvwmsj8kyrvf644p05r0hu79namyj3ens87650k"
                        .to_string(),
                    staking: "juno1v0km8gytmzpmtnwv8mpx26kctt5szuzudhg209fxee57yh9u2cvs88rn7p"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno1p3eed298qx3nyhs3grld07jrf9vjsjsmdd2kmmh3crk87emjcx5stp409y"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: None,
                },
                posthuman: DaoAddress {
                    cw20: "juno1rws84uz7969aaa7pej303udhlkt3j9ca0l3egpcae98jwak9quzq8szn2l"
                        .to_string(),
                    staking: "juno1jktfdt5g2d0fguvy8r8pl4gly7wps8phkwy08z6upc4nazkumrwq7lj0vn"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno17jv00cm4f3twr548jzayu57g9txvd4zdh54mdg9qpjs7samlphjsykylsq"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: None,
                },
                kleomedes: DaoAddress {
                    cw20: "juno10gthz5ufgrpuk5cscve2f0hjp56wgp90psqxcrqlg4m9mcu9dh8q4864xy"
                        .to_string(),
                    staking: "juno1zqp6uh3eg09s0h24rkwukkkg3pch49g0ndc53z9l8wrvh9dhf4nsj0ur49"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno1dpqgt3ja2kdxs94ltjw9ncdsexts9e3dx5qpnl20zvgdguzjelhqstf8zg"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: None,
                },
                cannalabs: DaoAddress {
                    cw20: "juno1vn38rzq0wc7zczp4dhy0h5y5kxh2jjzeahwe30c9cc6dw3lkyk5qn5rmfa"
                        .to_string(),
                    staking: "juno1066vq5g9qdprhgjst444rgf0zknhlqwmdnm7xyqhprt9whctzzxqdx90lu"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno17ckp36lmgtt7jtuggdv2j39eh4alcnl35szu6quh747nujags07swwq0nh"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: Some(
                        "juno1ls5un4a8zyn4f05k0ekq5aa9uhn88y8362ww38elqfpcwllme0jqelamke"
                            .to_string(),
                    ),
                },
                muse: DaoAddress {
                    cw20: "juno1p8x807f6h222ur0vssqy3qk6mcpa40gw2pchquz5atl935t7kvyq894ne3"
                        .to_string(),
                    staking: "juno17gdhjxt2d5mhx6paxc85g4pr5myew8pq0lm7usdsavsfk34ldrsqqhtafc"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno1rcssjyqgr6vzalss77d43v30c2qpyzzg607ua8gte2shqgtvu24sg8gs8r"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: None,
                },
            },
            spark_ibc: juno_destinations::comp_prefs::SparkIbcAddresses {
                fund: "juno1a6rna5tcl6p97rze6hnd5ug35kadqhudvr5f4mtr6s0yd5mruhss8gzrdy".to_string(),
            },
            balance_dao: "juno1ve7y09kvvnjk0yc2ycaq0y9thq5tct5ve6c0a5hfkt0h4jfy936qxtne5s"
                .to_string(),
            white_whale: juno_destinations::comp_prefs::WhiteWhaleSatelliteAddresses {
                amp_whale: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
                        .to_string(),
               
                bone_whale: 
                    "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
                        .to_string(),
               
                market: "juno1n8slcc79dmwuzdxhsesvhcncaqfg9h4czdm5t5ey8x25ajmn3xzqyde4wv"
                    .to_string(),
                rewards: "juno184ghwgprva7dlr2hwhzgvt6mem6zx78fygk0cpw7klssmzyf67tqdtwt3h"
                    .to_string(),
                    
                    juno_amp_whale_path:  vec![
                        // swap juno for usdc
                        SwapOperation::TerraSwap {
                            offer_asset_info: WWAssetInfo::NativeToken { denom: "ujuno".to_string() },
    
                            ask_asset_info: WWAssetInfo::NativeToken {
                                denom:
                                "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                            }
                        },
                        // usdc to whale
                        SwapOperation::TerraSwap {
                            offer_asset_info:  WWAssetInfo::NativeToken {
                                denom:
                                "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                            },
                            ask_asset_info: WWAssetInfo::NativeToken {
                                denom:
                                "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                            }
                        },
                        // whale to ampwhale
                        SwapOperation::TerraSwap {
                            offer_asset_info:  WWAssetInfo::NativeToken {
                                denom:
                                "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                            },
                            ask_asset_info: WWAssetInfo::NativeToken {
                                denom:
                                "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF".to_string()
                            }
                        },
                    ],
                    
    
                    juno_bone_whale_path: 
                        vec![
                            // swap juno for usdc
                            SwapOperation::TerraSwap {
                                offer_asset_info: WWAssetInfo::NativeToken { denom: "ujuno".to_string() },
    
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom:
                                    "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                                }
                            },
                            // usdc to whale
                            SwapOperation::TerraSwap {
                                offer_asset_info:  WWAssetInfo::NativeToken {
                                    denom:
                                    "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom:
                                    "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                                }
                            },
                            // whale to ampwhale
                            SwapOperation::TerraSwap {
                                offer_asset_info:  WWAssetInfo::NativeToken {
                                    denom:
                                    "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                                },
                                ask_asset_info: WWAssetInfo::NativeToken {
                                    denom:
                                    "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF".to_string()
                                }
                            },
                        ],
                        usdc_amp_whale_path:  vec![
                    
                    
                    // usdc to whale
                    SwapOperation::TerraSwap {
                        offer_asset_info:  WWAssetInfo::NativeToken {
                            denom:
                            "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                        },
                        ask_asset_info: WWAssetInfo::NativeToken {
                            denom:
                            "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                        }
                    },
                    // whale to ampwhale
                    SwapOperation::TerraSwap {
                        offer_asset_info:  WWAssetInfo::NativeToken {
                            denom:
                            "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                        },
                        ask_asset_info: WWAssetInfo::NativeToken {
                            denom:
                            "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF".to_string()
                        }
                    },
                ],
                

                usdc_bone_whale_path: 
                    vec![
                       
                        // usdc to whale
                        SwapOperation::TerraSwap {
                            offer_asset_info:  WWAssetInfo::NativeToken {
                                denom:
                                "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                            },
                            ask_asset_info: WWAssetInfo::NativeToken {
                                denom:
                                "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                            }
                        },
                        // whale to ampwhale
                        SwapOperation::TerraSwap {
                            offer_asset_info:  WWAssetInfo::NativeToken {
                                denom:
                                "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                            },
                            ask_asset_info: WWAssetInfo::NativeToken {
                                denom:
                                "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF".to_string()
                            }
                        },
                    ],

                terraswap_multihop_router:
                    "juno128lewlw6kv223uw4yzdffl8rnh3k9qs8vrf6kef28579w8ygccyq7m90n2".to_string(),
                // juno_bone_whale_path: todo!(),
            },
            racoon_bet: juno_destinations::comp_prefs::RacoonBetAddresses {
                game: "juno1h8p0jmfn06nfqpn0medn698h950vnl7v54m2azkyjdqjlzcly7jszxh7yu".to_string(),
                juno_usdc_wynd_pair:
                    "juno1gqy6rzary8vwnslmdavqre6jdhakcd4n2z4r803ajjmdq08r66hq7zcwrj".to_string(),
            },
            juno_lsds: juno_destinations::comp_prefs::JunoLsdAddresses {
                bone_juno: "juno102at0mu2xeluyw9efg257yy6pyhv088qqhmp4f8wszqcwxnpdcgqsfq0nv"
                    .to_string(),
                wy_juno: "juno18wuy5qr2mswgz7zak8yr9crhwhtur3v6mw4tcytupywxzw7sufyqgza7uh"
                    .to_string(),
                se_juno: "juno1dlp8avgc2r6t4nnsv4yydc6lc73rjtjqvdcee9r2kf0uwuef7v0smljy8w"
                    .to_string(),
                b_juno: "juno1dlp8avgc2r6t4nnsv4yydc6lc73rjtjqvdcee9r2kf0uwuef7v0smljy8w"
                    .to_string(),
                amp_juno: "juno17cya4sw72h4886zsm2lk3udxaw5m8ssgpsl6nd6xl6a4ukepdgkqeuv99x"
                    .to_string(),
            },
        },
    };
    let wyndstake_project_addresses = ymos_wyndstake_outpost::msg::ContractAddresses {
        take_rate_addr:junostake_project_addresses.take_rate_addr.clone(),
        usdc: junostake_project_addresses.usdc.clone(),
        authzpp: ymos_wyndstake_outpost::msg::AuthzppAddresses::default(),
        destination_projects: junostake_project_addresses.destination_projects.clone(),
        wynd_stake_addr: "juno1sy9mlw47w44f94zea7g98y5ff4cvtc8rfv75jgwphlet83wlf4ssa050mv".to_string()
    };
    let junodca_project_addresses = ymos_junodca_outpost::msg::ContractAddresses {
        take_rate_addr: junostake_project_addresses.take_rate_addr.clone(),
        usdc: junostake_project_addresses.usdc.clone(),
        authzpp: ymos_junodca_outpost::msg::AuthzppAddresses::default(),
        destination_projects: junostake_project_addresses.destination_projects.clone(),
    };
    let junowwmarket_project_addresses = ymos_junowwmarket_outpost::msg::ContractAddresses {
        take_rate_addr: junostake_project_addresses.take_rate_addr.clone(),
        usdc: junostake_project_addresses.usdc.clone(),
        authzpp: ymos_junowwmarket_outpost::msg::AuthzppAddresses::default(),
        destination_projects: junostake_project_addresses.destination_projects.clone(),
        
        terraswap_routes: TerraswapRouteAddresses { whale_usdc_pool: "juno1g7ctm7dynjsduf597d8nvt36kwvhfutmzrczdnm00tsz48uryvzqp7p32h".to_string(), 
            whale_to_juno_route: vec![
                //  whale to usdc
                SwapOperation::TerraSwap {
                    ask_asset_info:  WWAssetInfo::NativeToken {
                        denom:
                        "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                    },
                    offer_asset_info: WWAssetInfo::NativeToken {
                        denom:
                        "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                    }
                },
                // swap usdc for juno
                SwapOperation::TerraSwap {
                    ask_asset_info: WWAssetInfo::NativeToken { denom: "ujuno".to_string() },

                    offer_asset_info: WWAssetInfo::NativeToken {
                        denom:
                        "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                    }
                },
                
            ],
            whale_to_atom_route: vec![//  whale to usdc
                SwapOperation::TerraSwap {
                    ask_asset_info:  WWAssetInfo::NativeToken {
                        denom:
                        "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                    },
                    offer_asset_info: WWAssetInfo::NativeToken {
                        denom:
                        "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()
                    }
                },SwapOperation::TerraSwap {
                    offer_asset_info:  WWAssetInfo::NativeToken {
                        denom:
                        "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string()
                    },
                    ask_asset_info: WWAssetInfo::NativeToken {
                        denom:
                        "ibc/D8D6271EC54E3A96C6B9FB6C2BA9E99692B07CEB42754638029657072EA48337".to_string()
                    }
                },            
            ],

             whale_ampwhale_pool: "juno1dwmrkyhed4szdxxk6l0c98hseancjtdet58n77tfhv2as8cdjdlq7vps00".to_string(), 
             whale_bonewhale_pool: "juno160uh2xtegzvc7ekte5x377aud0y40hw75m9l92h7pkqk3l3eg9vqltel48".to_string(),
             whale_rac_pool: "juno1qv337g245ger3cx294m3vu74z74ku7lpf4944qxf8nhx29s8568q4uwrmk".to_string(),
            
            usdc_asset: WWAssetInfo::NativeToken { denom: "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string() },
            ampwhale_asset: WWAssetInfo::NativeToken { denom: "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
            .to_string() },
            bonewhale_asset: WWAssetInfo::NativeToken { denom: "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
            .to_string() },
            juno_asset: WWAssetInfo::NativeToken { denom: "ujuno".to_string() },
            whale_asset: WWAssetInfo::NativeToken {
                denom: "ibc/3A6ADE78FB8169C034C29C4F2E1A61CE596EC8235366F22381D981A98F1F5A5C".to_string()},
            
            rac_asset: WWAssetInfo::NativeToken { denom: "ibc/D8D6271EC54E3A96C6B9FB6C2BA9E99692B07CEB42754638029657072EA48337".to_string() },
            atom_asset: WWAssetInfo::NativeToken { denom: "ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9".to_string() }
        },
        
    };

    let rt = Runtime::new().unwrap();
    dotenv::dotenv().ok();
    env_logger::init();

    let juno: ChainInfo = ChainInfo {
        kind: ChainKind::Mainnet,
        chain_id: "juno-1",
        gas_denom: "ujuno",
        gas_price: 0.0750,
        grpc_urls: &[
            // "https://grpc-juno-ia.cosmosia.notional.ventures",
            "http://juno-grpc.polkachu.com:12690",
        ],
        network_info: JUNO_NETWORK,
        lcd_url: None,
        fcd_url: None,
    };

    let juno_chain = DaemonBuilder::default()
        .handle(rt.handle())
        .chain(juno.clone())
        .build()?;

    println!("connected to juno with sender: {}", juno_chain.sender());

    let juno_comp_prefs = YmosCompPrefsContract::new(
        "Yieldmos Juno Compounding Preferences",
        juno_chain.clone(),
    );

    let junostake = ymos_junostake_outpost::YmosJunostakeOutpost::new(
        "Yieldmos Junostake Outpost",
        juno_chain.clone(),
    );
    let wyndstake = ymos_wyndstake_outpost::YmosWyndstakeOutpost::new(
        "Yieldmos Wyndstake Outpost",
        juno_chain.clone(),
    );
    let junodca =
        ymos_junodca_outpost::YmosJunodcaOutpost::new("Yieldmos Juno DCA Outpost", juno_chain.clone());
    
        let junowwmarket =
        ymos_junowwmarket_outpost::YmosJunowwmarketOutpost::new("Yieldmos Juno White Whale Market Outpost", juno_chain.clone());


    


    juno_comp_prefs.upload_if_needed()?;

    junodca.upload_if_needed()?;
    println!("junodca code id: {}", junodca.code_id()?);

    junostake.upload_if_needed()?;
    println!("junostake code id: {}", junostake.code_id()?);

    wyndstake.upload_if_needed()?;
    println!("wyndstake code id: {}", wyndstake.code_id()?);

    junowwmarket.upload_if_needed()?;
    println!("junowwmarket code id: {}", junowwmarket.code_id()?);

    

    // juno_comp_prefs contract upload
    if juno_comp_prefs.address().is_err() {
        juno_comp_prefs.instantiate(
            &ymos_comp_prefs::msg::InstantiateMsg {
                admin: None,
                chain_id:"juno-1".to_string(),
                days_to_prune: 180u16,
            },
            Some(&Addr::unchecked(juno_chain.sender().to_string())),
            None,
        )?;

        // dca
        juno_comp_prefs
            .add_allowed_strategy_id(Uint64::from(60100u64))?;
        // juno staking
            juno_comp_prefs
            .add_allowed_strategy_id(Uint64::from(60101u64))?;
        // wynd stake
        juno_comp_prefs
            .add_allowed_strategy_id(Uint64::from(60102u64))?;
        // white whale sat market 
        juno_comp_prefs
            .add_allowed_strategy_id(Uint64::from(60103u64))?;

        // setup the feeshare only on the first deploy
        // this seems to sometimes need an increased gas multiplier in the .env to work
        // juno_chain
        //     .commit_any::<cosmrs::Any>(
        //         vec![feeshare_msg(
        //             juno_comp_prefs.address().unwrap().to_string(),
        //             juno_chain.sender().to_string(),
        //             juno_chain.sender().to_string(),
        //         )],
        //         None,
        //     )
        //     .unwrap();
    } else {
        juno_comp_prefs.migrate(
            &ymos_comp_prefs::msg::MigrateMsg {
               
            },
            juno_comp_prefs.code_id()?,
        )?;
    }
    println!("juno_comp_prefs: {}", juno_comp_prefs.addr_str()?);

    // junostake contract upload
    if junostake.address().is_err() {
        junostake.instantiate(
            &ymos_junostake_outpost::msg::InstantiateMsg {
                admin: Some(juno_chain.sender().to_string()),
                project_addresses: junostake_project_addresses.clone(),
            },
            Some(&Addr::unchecked(juno_chain.sender().to_string())),
            None,
        )?;

        // add yieldmos.juno as an authorized compounder
        junostake
            .add_authorized_compounder(YMOS_CONDUCTOR.to_string())
            .unwrap();

        // setup the feeshare only on the first deploy
        // this seems to sometimes need an increased gas multiplier in the .env to work
        // juno_chain
        //     .commit_any::<cosmrs::Any>(
        //         vec![feeshare_msg(
        //             junostake.address().unwrap().to_string(),
        //             juno_chain.sender().to_string(),
        //             juno_chain.sender().to_string(),
        //         )],
        //         None,
        //     )
        //     .unwrap();
    } else {
        junostake.migrate(
            &ymos_junostake_outpost::msg::MigrateMsg {
                project_addresses: Some(junostake_project_addresses.clone()),
            },
            junostake.code_id()?,
        )?;
    }
    println!("junostake: {}", junostake.addr_str()?);

    // junodca contract upload
    if junodca.address().is_err() {
        junodca.instantiate(
            &ymos_junodca_outpost::msg::InstantiateMsg {
                admin: Some(juno_chain.sender().to_string()),
                project_addresses: junodca_project_addresses.clone(),
            },
            Some(&Addr::unchecked(juno_chain.sender().to_string())),
            None,
        )?;

        // setup the feeshare only on the first deploy
        // this seems to sometimes need an increased gas multiplier in the .env to work
        // juno_chain
        //     .commit_any::<cosmrs::Any>(
        //         vec![feeshare_msg(
        //             junodca.address().unwrap().to_string(),
        //             juno_chain.sender().to_string(),
        //             juno_chain.sender().to_string(),
        //         )],
        //         None,
        //     )
        //     .unwrap();

        // add yieldmos.juno as an authorized compounder
        junodca
            .add_authorized_compounder(YMOS_CONDUCTOR.to_string())
            .unwrap();
    } else {
        junodca.migrate(
            &ymos_junodca_outpost::msg::MigrateMsg {
                project_addresses: Some(junodca_project_addresses.clone()),
            },
            junodca.code_id()?,
        )?;
    }
    println!("junodca: {}", junodca.addr_str()?);


    // wyndstake contract upload
    if wyndstake.address().is_err() {
        wyndstake.instantiate(
            &ymos_wyndstake_outpost::msg::InstantiateMsg {
                admin: Some(juno_chain.sender().to_string()),
                project_addresses: wyndstake_project_addresses.clone(),
            },
            Some(&Addr::unchecked(juno_chain.sender().to_string())),
            None,
        )?;

        // add yieldmos.juno as an authorized compounder
        wyndstake
            .add_authorized_compounder(YMOS_CONDUCTOR.to_string())
            .unwrap();

        // setup the feeshare only on the first deploy
        // this seems to sometimes need an increased gas multiplier in the .env to work
        // juno_chain
        //     .commit_any::<cosmrs::Any>(
        //         vec![feeshare_msg(
        //             wyndstake.address().unwrap().to_string(),
        //             juno_chain.sender().to_string(),
        //             juno_chain.sender().to_string(),
        //         )],
        //         None,
        //     )
        //     .unwrap();
    } else {
        wyndstake.migrate(
            &ymos_wyndstake_outpost::msg::MigrateMsg {
                project_addresses: Some(wyndstake_project_addresses.clone()),
            },
            wyndstake.code_id()?,
        )?;
    }
    println!("wyndstake: {}", wyndstake.addr_str()?);


    // junowwmarket contract upload
    if junowwmarket.address().is_err() {
        junowwmarket.instantiate(
            &ymos_junowwmarket_outpost::msg::InstantiateMsg {
                admin: Some(juno_chain.sender().to_string()),
                project_addresses: junowwmarket_project_addresses.clone(),
            },
            Some(&Addr::unchecked(juno_chain.sender().to_string())),
            None,
        )?;

        // add yieldmos.juno as an authorized compounder
        junowwmarket
            .add_authorized_compounder(YMOS_CONDUCTOR.to_string())
            .unwrap();

        // setup the feeshare only on the first deploy
        // this seems to sometimes need an increased gas multiplier in the .env to work
        // juno_chain
        //     .commit_any::<cosmrs::Any>(
        //         vec![feeshare_msg(
        //             junowwmarket.address().unwrap().to_string(),
        //             juno_chain.sender().to_string(),
        //             juno_chain.sender().to_string(),
        //         )],
        //         None,
        //     )
        //     .unwrap();
    } else {
        junowwmarket.migrate(
            &ymos_junowwmarket_outpost::msg::MigrateMsg {
                project_addresses: Some(junowwmarket_project_addresses.clone()),
            },
            junowwmarket.code_id()?,
        )?;
    }
    println!("junowwmarket: {}", junowwmarket.addr_str()?);

    Ok(())
}

pub fn feeshare_msg(
    contract_address: String,
    deployer_address: String,
    withdrawer_address: String,
) -> cosmrs::Any {
    cosmrs::Any {
        type_url: "/juno.feeshare.v1.MsgRegisterFeeShare".to_string(),
        value: Anybuf::new()
            .append_string(1, contract_address)
            .append_string(2, deployer_address)
            .append_string(3, withdrawer_address)
            .into_vec(),
    }
}
