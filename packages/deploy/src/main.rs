use anybuf::Anybuf;
// use cw_orch::prelude::interchain_channel_builder::InterchainChannelBuilder;
// use cw_orch::starship::Starship;
use cw_orch::{anyhow, daemon::DaemonBuilder, prelude::*};
use outpost_utils::juno_comp_prefs::DaoAddress;
// use spark_ibc_sender::state::{Config, SparkContractAddr};
use tokio::runtime::Runtime;

#[derive(PartialEq, Eq, Debug)]
pub enum DeploymentType {
    Prod,
    Dev,
}

pub fn main() -> anyhow::Result<()> {
    let junostake_project_addresses = ymos_junostake_outpost::msg::ContractAddresses {
        // needs to be switchout for mainnet
        take_rate_addr: "juno1twfv52yxcyykx2lcvgl42svw46hsm5ddhq6u2f".to_string(),
        usdc: wyndex::asset::AssetInfo::Native(
            "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034".to_string(),
        ),
        authzpp: ymos_junostake_outpost::msg::AuthzppAddresses {
            withdraw_tax: "juno1nak433pjd39et4g6jjclxk7yfmtfsd5m43su04rxe9ggttdvjwpqsumv30"
                .to_string(),
        },
        destination_projects: outpost_utils::juno_comp_prefs::DestinationProjectAddresses {
            wynd: outpost_utils::juno_comp_prefs::WyndAddresses {
                cw20: "juno1mkw83sv6c7sjdvsaplrzc8yaes9l42p4mhy0ssuxjnyzl87c9eps7ce3m9".to_string(),
                multihop: "juno1pctfpv9k03v0ff538pz8kkw5ujlptntzkwjg6c0lrtqv87s9k28qdtl50w"
                    .to_string(),
                juno_wynd_pair: "juno1a7lmc8e04hcs4y2275cultvg83u636ult4pmnwktr6l9nhrh2e8qzxfdwf"
                    .to_string(),
            },
            gelotto: outpost_utils::juno_comp_prefs::GelottoAddresses {
                pick3_contract: "juno1v466lyrhsflkt9anxt4wyx7mtw8w2uyk0qxkqskqfj90rmwhph7s0dxvga"
                    .to_string(),
                pick4_contract: "juno16xy5m05z6n4vnfzcf8cvd3anxhg4g2k8vvr4q2knv4akynfstr9qjmhdhs"
                    .to_string(),
                pick5_contract: "juno1txn3kejj4qrehua9vlg3hk4wunqafqunfy83cz5hg2xa3z3pkgssk4tzu4"
                    .to_string(),
            },
            daos: outpost_utils::juno_comp_prefs::DaoAddresses {
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
                    staking: "juno14lycavan8gvpjn97aapzvwmsj8kyrvf644p05r0hu79namyj3ens87650k"
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
                    staking: "juno1rws84uz7969aaa7pej303udhlkt3j9ca0l3egpcae98jwak9quzq8szn2l"
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
                    staking: "juno10gthz5ufgrpuk5cscve2f0hjp56wgp90psqxcrqlg4m9mcu9dh8q4864xy"
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
                    staking: "juno1vn38rzq0wc7zczp4dhy0h5y5kxh2jjzeahwe30c9cc6dw3lkyk5qn5rmfa"
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
                    staking: "juno1p8x807f6h222ur0vssqy3qk6mcpa40gw2pchquz5atl935t7kvyq894ne3"
                        .to_string(),
                    juno_wyndex_pair: Some(
                        "juno1rcssjyqgr6vzalss77d43v30c2qpyzzg607ua8gte2shqgtvu24sg8gs8r"
                            .to_string(),
                    ),
                    wynd_wyndex_pair: None,
                },
            },
            spark_ibc: outpost_utils::juno_comp_prefs::SparkIbcAddresses {
                fund: "juno1a6rna5tcl6p97rze6hnd5ug35kadqhudvr5f4mtr6s0yd5mruhss8gzrdy".to_string(),
            },
            balance_dao: "juno1ve7y09kvvnjk0yc2ycaq0y9thq5tct5ve6c0a5hfkt0h4jfy936qxtne5s"
                .to_string(),
            white_whale: outpost_utils::juno_comp_prefs::WhiteWhaleSatelliteAddresses {
                amp_whale: wyndex::asset::AssetInfo::Native(
                    "ibc/2F7C2A3D5D42553ED46F57D8B0DE3733B1B5FF571E2C6A051D34525904B4C0AF"
                        .to_string(),
                ),
                bone_whale: wyndex::asset::AssetInfo::Native(
                    "ibc/01BAE2E69D02670B22758FBA74E4114B6E88FC1878936C919DA345E6C6C92ABF"
                        .to_string(),
                ),
                market: "juno1n8slcc79dmwuzdxhsesvhcncaqfg9h4czdm5t5ey8x25ajmn3xzqyde4wv"
                    .to_string(),
                rewards: "juno184ghwgprva7dlr2hwhzgvt6mem6zx78fygk0cpw7klssmzyf67tqdtwt3h"
                    .to_string(),
            },
            racoon_bet: outpost_utils::juno_comp_prefs::RacoonBetAddresses {
                game: "juno1h8p0jmfn06nfqpn0medn698h950vnl7v54m2azkyjdqjlzcly7jszxh7yu".to_string(),
                juno_usdc_wynd_pair:
                    "juno1gqy6rzary8vwnslmdavqre6jdhakcd4n2z4r803ajjmdq08r66hq7zcwrj".to_string(),
            },
            juno_lsds: outpost_utils::juno_comp_prefs::JunoLsdAddresses {
                bone_juno: "juno102at0mu2xeluyw9efg257yy6pyhv088qqhmp4f8wszqcwxnpdcgqsfq0nv"
                    .to_string(),
                wy_juno: "juno18wuy5qr2mswgz7zak8yr9crhwhtur3v6mw4tcytupywxzw7sufyqgza7uh"
                    .to_string(),
                se_juno: "juno1dd0k0um5rqncfueza62w9sentdfh3ec4nw4aq4lk5hkjl63vljqscth9gv"
                    .to_string(),
                b_juno: "juno1wwnhkagvcd3tjz6f8vsdsw5plqnw8qy2aj3rrhqr2axvktzv9q2qz8jxn3"
                    .to_string(),
                amp_juno: "juno17cya4sw72h4886zsm2lk3udxaw5m8ssgpsl6nd6xl6a4ukepdgkqeuv99x"
                    .to_string(),
            },
        },
    };

    let rt = Runtime::new().unwrap();
    dotenv::dotenv().ok();
    env_logger::init();

    let juno = networks::JUNO_1;

    let juno_chain = DaemonBuilder::default()
        .handle(rt.handle())
        .chain(juno.clone())
        .build()?;

    println!("connected to juno with sender: {}", juno_chain.sender());

    let junostake = ymos_junostake_outpost::YmosJunostakeOutpost::new(
        "ymos_junostake_address",
        juno_chain.clone(),
    );

    junostake.upload_if_needed()?;
    println!("junostake code id: {}", junostake.code_id()?);

    // single spark points ledger on juno
    if junostake.address().is_err() {
        junostake.instantiate(
            &ymos_junostake_outpost::msg::InstantiateMsg {
                admin: Some(juno_chain.sender().to_string()),
                project_addresses: junostake_project_addresses.clone(),
            },
            Some(&Addr::unchecked(juno_chain.sender().to_string())),
            None,
        )?;
    } else {
        junostake.migrate_if_needed(&ymos_junostake_outpost::msg::InstantiateMsg {
            admin: Some(juno_chain.sender().to_string()),
            project_addresses: junostake_project_addresses.clone(),
        })?;
    }
    println!("junostake: {}", junostake.addr_str()?);

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
