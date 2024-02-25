use cosmwasm_std::Uint128;
use osmosis_destinations::{
    comp_prefs::DestProjectSwapRoutes,
    pools::{Denoms, OsmoPools, OsmosisKnownPoolListing, UsdcPools},
};
use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute;

use crate::osmosis_swap::{unsafe_generate_known_to_known_route, KnownRoutePools};

#[test]
fn generate_known_to_known_routes() {
    let denoms: Denoms = Denoms {
        usdc: "ibc/uusdc".to_string(),
        axlusdc: "ibc/uaxlusdc".to_string(),
        osmo: "ibc/uosmo".to_string(),
        ion: "ibc/uion".to_string(),
        tia: "ibc/utia".to_string(),
        atom: "ibc/uatom".to_string(),
        amp_osmo: "ibc/uamp_osmo".to_string(),
        mars: "ibc/umars".to_string(),
        whale: "ibc/uwhale".to_string(),
        amp_whale: "ibc/uamp_whale".to_string(),
        mbrn: "ibc/umbrn".to_string(),
        cdt: "ibc/ucdt".to_string(),
    };

    let osmo_pools = OsmoPools {
        tia: OsmosisKnownPoolListing {
            pool_id: 1,
            out_denom: "ibc/utia".to_string(),
        },
        ion: OsmosisKnownPoolListing {
            pool_id: 2,
            out_denom: "ibc/uion".to_string(),
        },
        mars: OsmosisKnownPoolListing {
            pool_id: 3,
            out_denom: "ibc/umars".to_string(),
        },
        usdc: OsmosisKnownPoolListing {
            pool_id: 4,
            out_denom: "ibc/uusdc".to_string(),
        },
        atom: OsmosisKnownPoolListing {
            pool_id: 5,
            out_denom: "ibc/uatom".to_string(),
        },
        whale: OsmosisKnownPoolListing {
            pool_id: 6,
            out_denom: "ibc/uwhale".to_string(),
        },
        mbrn: OsmosisKnownPoolListing {
            pool_id: 7,
            out_denom: "ibc/umbrn".to_string(),
        },
        cdt: OsmosisKnownPoolListing {
            pool_id: 8,
            out_denom: "ibc/ucdt".to_string(),
        },
    };

    let usdc_pools = UsdcPools {
        tia: OsmosisKnownPoolListing {
            pool_id: 11,
            out_denom: "ibc/utia".to_string(),
        },
        atom: OsmosisKnownPoolListing {
            pool_id: 12,
            out_denom: "ibc/uatom".to_string(),
        },
        osmo: OsmosisKnownPoolListing {
            pool_id: 4,
            out_denom: "ibc/uosmo".to_string(),
        },
        cdt: OsmosisKnownPoolListing {
            pool_id: 14,
            out_denom: "ibc/ucdt".to_string(),
        },
        axlusdc: OsmosisKnownPoolListing {
            pool_id: 15,
            out_denom: "ibc/uaxlusdc".to_string(),
        },
    };

    let pools = DestProjectSwapRoutes {
        osmo_pools,
        usdc_pools,
    };

    assert_eq!(
        unsafe_generate_known_to_known_route(
            &pools,
            &denoms,
            &denoms.osmo,
            &denoms.atom,
            KnownRoutePools {
                from_token_osmo_pool: None,
                to_token_osmo_pool: Some(5),
                from_token_usdc_pool: Some(13),
                to_token_usdc_pool: Some(12)
            }
        )
        .unwrap(),
        vec![SwapAmountInRoute {
            pool_id: 5,
            token_out_denom: denoms.atom.to_string(),
        }],
        "Should generate a known route from osmo to atom"
    );

    assert_eq!(
        unsafe_generate_known_to_known_route(
            &pools,
            &denoms,
            &denoms.usdc,
            &denoms.atom,
            KnownRoutePools {
                from_token_osmo_pool: Some(4),
                to_token_osmo_pool: Some(5),
                from_token_usdc_pool: None,
                to_token_usdc_pool: Some(12)
            }
        )
        .unwrap(),
        vec![SwapAmountInRoute {
            pool_id: 12,
            token_out_denom: denoms.atom.to_string(),
        }],
        "Should generate a known route from usdc to atom"
    );

    assert_eq!(
        unsafe_generate_known_to_known_route(
            &pools,
            &denoms,
            &denoms.mbrn,
            &denoms.whale,
            KnownRoutePools {
                from_token_osmo_pool: Some(7),
                to_token_osmo_pool: Some(6),
                from_token_usdc_pool: None,
                to_token_usdc_pool: None
            }
        )
        .unwrap(),
        vec![
            SwapAmountInRoute {
                pool_id: 7,
                token_out_denom: denoms.osmo.to_string(),
            },
            SwapAmountInRoute {
                pool_id: 6,
                token_out_denom: denoms.whale.to_string(),
            }
        ],
        "Should generate a known route from mbrn to whale via osmo"
    );

    assert_eq!(
        unsafe_generate_known_to_known_route(
            &pools,
            &denoms,
            &denoms.cdt,
            &denoms.axlusdc,
            KnownRoutePools {
                from_token_osmo_pool: Some(8),
                to_token_osmo_pool: None,
                from_token_usdc_pool: Some(14),
                to_token_usdc_pool: Some(15),
            }
        )
        .unwrap(),
        vec![
            SwapAmountInRoute {
                pool_id: 14,
                token_out_denom: denoms.usdc.to_string(),
            },
            SwapAmountInRoute {
                pool_id: 15,
                token_out_denom: denoms.axlusdc.to_string(),
            }
        ],
        "Should generate a known route from cdt to axlusdc via usdc"
    );

    assert_eq!(
        unsafe_generate_known_to_known_route(
            &pools,
            &denoms,
            &denoms.mars,
            &denoms.axlusdc,
            KnownRoutePools {
                from_token_osmo_pool: Some(3),
                to_token_osmo_pool: None,
                from_token_usdc_pool: None,
                to_token_usdc_pool: Some(15),
            }
        )
        .unwrap(),
        vec![
            SwapAmountInRoute {
                pool_id: 3,
                token_out_denom: denoms.osmo.to_string(),
            },
            SwapAmountInRoute {
                pool_id: 4,
                token_out_denom: denoms.usdc.to_string(),
            },
            SwapAmountInRoute {
                pool_id: 15,
                token_out_denom: denoms.axlusdc.to_string(),
            }
        ],
        "Should generate a known route from MARS to axlUSDC via osmo and then usdc"
    );

    assert_eq!(
        unsafe_generate_known_to_known_route(
            &pools,
            &denoms,
            &denoms.axlusdc,
            &denoms.ion,
            KnownRoutePools {
                from_token_osmo_pool: None,
                to_token_osmo_pool: Some(2),
                from_token_usdc_pool: Some(15),
                to_token_usdc_pool: None,
            }
        )
        .unwrap(),
        vec![
            SwapAmountInRoute {
                pool_id: 15,
                token_out_denom: denoms.usdc.to_string(),
            },
            SwapAmountInRoute {
                pool_id: 4,
                token_out_denom: denoms.osmo.to_string(),
            },
            SwapAmountInRoute {
                pool_id: 2,
                token_out_denom: denoms.ion.to_string(),
            }
        ],
        "Should generate a known route from axlUSDC to ION via usdc and then osmo"
    );
}
