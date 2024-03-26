use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, ReplyOn, Storage, Timestamp, Uint128,
};
use cw_storage_plus::{Item, Map};
use outpost_utils::{comp_prefs::store_submsg_data, msg_gen::CosmosProtoMsg};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    comp_prefs::{MembraneAddrs, MembraneDepositCollateralAction, OsmosisPoolSettings},
    dest_project_gen::{
        deposit_into_cdp_msgs, deposit_into_stability_pool_msgs, mint_cdt_msgs, DestinationResult,
    },
    errors::OsmosisDestinationError,
};
use membrane::{cdp, types::UserInfo};

#[cw_serde]
pub enum MembraneStabilityPoolExecuteMsg {
    /// Deposit the debt token into the pool
    Deposit {
        /// User address, defaults to info.sender
        user: Option<String>,
    },
    /// Claim ALL liquidation revenue && MBRN incentives
    /// Can be queried from UserClaims
    ClaimRewards {},
}

#[cw_serde]
pub enum MembraneStakingExecuteMsg {
    Stake { user: Option<String> },
}

#[cw_serde]
pub enum MembraneCDPExecuteMsg {
    Deposit {
        /// Position ID to deposit into.
        /// If the user wants to create a new/separate position, no position id is passed.
        position_id: Option<Uint128>,
        /// Position owner.
        /// Defaults to the sender.
        position_owner: Option<String>,
    },
    // CDP Mint CDT
    IncreaseDebt {
        /// Position ID to increase debt of
        position_id: Uint128,
        /// Amount of debt to increase
        amount: Option<Uint128>,
        /// LTV to borrow up to
        LTV: Option<Decimal>,
        /// Mint debt tokens to this address
        mint_to_addr: Option<String>,
    },
    // CDP Repay position debt
    Repay {
        /// Position ID to repay debt of
        position_id: Uint128,
        /// Position owner to repay debt of if not the sender
        position_owner: Option<String>,
        /// Send excess assets to this address if not the sender
        send_excess_to: Option<String>,
    },
}

#[cw_serde]
pub enum MembraneCDPQueryMsg {
    /// Returns the contract's Basket
    GetBasket {},
}

/// Deposit collateral into a CDP and then do something else
/// This may spawn a submessage and if it does `membrane_mint_cdt` can be used to handle the followup
pub fn membrane_deposit_collateral_and_then<T>(
    store: &mut dyn Storage,
    user_addr: &Addr,
    cdp_addr: &Addr,
    position_id: Uint128,
    deposit_assets: &Vec<Coin>,
    and_then: MembraneDepositCollateralAction,
    submsg_data: T,
    latest_reply_id_state: Item<u64>,
    submsg_state: Map<&u64, T>,
) -> DestinationResult
where
    T: Serialize + DeserializeOwned,
{
    // if the action is to mint CDT we dont need a submsg and we can
    // just deposit the assets and mint however much CDT is needed
    if let MembraneDepositCollateralAction::MintCdt { desired_ltv } = and_then {
        let mut deposit_msg =
            deposit_into_cdp_msgs(user_addr, &cdp_addr, position_id, deposit_assets, None)?;

        deposit_msg.append_msgs(mint_cdt_msgs(
            user_addr,
            cdp_addr,
            position_id,
            desired_ltv,
        )?);

        Ok(deposit_msg)
    // if the action is not to mint CDT we need to store the submsg data
    // so that we can query how much CDT to mint when the submsg returns
    } else {
        deposit_into_cdp_msgs(
            user_addr,
            &cdp_addr,
            position_id,
            deposit_assets,
            Some((
                store_submsg_data(store, submsg_data, latest_reply_id_state, submsg_state)?,
                ReplyOn::Success,
            )),
        )
    }
}

/// Mint CDT and then do something else.
/// This function is intended to be able to accurately query how much CDT to mint
/// and thus cannot be used directly after adding collateral to a CDP.
pub fn membrane_mint_cdt(
    querier: &QuerierWrapper,
    membrane_addrs: &MembraneAddrs,
    cdt_denom: &str,
    user_addr: &Addr,
    position_id: Uint128,
    and_then: MembraneDepositCollateralAction,
    current_timestamp: Timestamp,
) -> DestinationResult {
    let desired_ltv = and_then.desired_ltv();
    let simulated_cdt: Uint128 = querier.query_wasm_smart(
        membrane_addrs.cdp,
        &cdp::QueryMsg::SimulateMint {
            position_info: UserInfo {
                position_id,
                position_owner: user_addr,
            },
            LTV: desired_ltv,
        },
    )?;
    // the amount of cdt we can expect to have minted based off our simulation
    let minted_cdt = coin(simulated_cdt, cdt_denom);

    let mut mint_cdt = mint_cdt_msgs(user_addr, &membrane_addrs.cdp, position_id, desired_ltv)?;

    match and_then {
        MembraneDepositCollateralAction::MintCdt { desired_ltv } => (),
        MembraneDepositCollateralAction::EnterStabilityPool { .. } => {
            mint_cdt.append_msgs(deposit_into_stability_pool_msgs(
                user_addr,
                &membrane_addrs.stability_pool,
                minted_cdt,
            )?);
        }
        MembraneDepositCollateralAction::ProvideLiquidity {
            pool_id,
            pool_settings: OsmosisPoolSettings::Standard { bond_tokens },
            ..
        } => {
            let lp_msgs: Vec<CosmosProtoMsg> =
                join_osmosis_pool_single_side(user_addr, pool_id, minted_cdt, bond_tokens)?;

            mint_cdt.append_msgs(lp_msgs);
        }
        MembraneDepositCollateralAction::ProvideLiquidity {
            pool_id,
            pool_settings:
                OsmosisPoolSettings::ConcentratedLiquidity {
                    lower_tick,
                    upper_tick,
                    token_min_amount_0,
                    token_min_amount_1,
                },
            ..
        } => {
            let lp_msgs: Vec<CosmosProtoMsg> = join_osmosis_cl_pool_single_side(
                querier,
                user_addr,
                pool_id,
                minted_cdt,
                bond_tokens,
                lower_tick,
                upper_tick,
                token_min_amount_0,
                token_min_amount_1,
            )?;

            mint_cdt.append_msgs(lp_msgs);
        }
    }
    Ok(mint_cdt)
}
