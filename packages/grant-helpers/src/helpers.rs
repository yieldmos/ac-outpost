use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct YmosAsset {
    pub amount: Uint128,
    pub info: YmosAssetInfo,
}

#[cw_serde]
pub enum YmosAssetInfo {
    Native(String),
    Token(String),
}

impl Display for YmosAssetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YmosAssetInfo::Native(denom) => write!(f, "{}", denom),
            YmosAssetInfo::Token(cw20_addr) => write!(f, "{}", cw20_addr),
        }
    }
}
