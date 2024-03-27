use crate::{
    errors::MembraneHelperError,
    msg_gen::{
        deposit_into_cdp_msgs, deposit_into_stability_pool_msgs, mint_cdt_msgs, DestinationResult,
    },
};
use cosmwasm_std::{coin, Addr, Coin, QuerierWrapper, ReplyOn, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use membrane::{
    cdp,
    types::{Basket, UserInfo},
};
use osmosis_destinations::comp_prefs::{
    MembraneAddrs, MembraneDepositCollateralAction, OsmosisPoolSettings,
};
use osmosis_helpers::osmosis_lp::{
    join_osmosis_cl_pool_single_side, join_osmosis_pool_single_side,
};
use outpost_utils::{comp_prefs::store_submsg_data, msg_gen::CosmosProtoMsg};
use serde::{de::DeserializeOwned, Serialize};

/// filter out assets that are not in the CDP basket
pub fn basket_denoms_filter(
    querier: &QuerierWrapper,
    cdp_contract_addr: &Addr,
    assets: &Vec<Coin>,
) -> Result<Vec<Coin>, MembraneHelperError> {
    // check the currently allowed assets
    let basket: Basket =
        querier.query_wasm_smart(cdp_contract_addr, &cdp::QueryMsg::GetBasket {})?;

    Ok(assets
        .into_iter()
        // filter out assets that are not in the basket
        .filter(|coin| {
            basket
                .collateral_types
                .iter()
                // very weird checking Asset against Coin.denom. might work might blow up
                .any(|asset| asset.asset.info.to_string().eq(&coin.denom))
        })
        .cloned()
        .collect())
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

        deposit_msg.concat_after(mint_cdt_msgs(
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
        membrane_addrs.cdp.clone(),
        &cdp::QueryMsg::SimulateMint {
            position_info: UserInfo {
                position_id,
                position_owner: user_addr.to_string(),
            },
            LTV: desired_ltv,
        },
    )?;
    // the amount of cdt we can expect to have minted based off our simulation
    let minted_cdt = coin(simulated_cdt.u128(), cdt_denom);

    let mut mint_cdt = mint_cdt_msgs(user_addr, &membrane_addrs.cdp, position_id, desired_ltv)?;

    match and_then {
        MembraneDepositCollateralAction::MintCdt { desired_ltv } => (),
        MembraneDepositCollateralAction::EnterStabilityPool { .. } => {
            mint_cdt.concat_after(deposit_into_stability_pool_msgs(
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
                lower_tick,
                upper_tick,
                token_min_amount_0,
                token_min_amount_1,
                current_timestamp,
            )?;

            mint_cdt.append_msgs(lp_msgs);
        }
    }
    Ok(mint_cdt)
}
