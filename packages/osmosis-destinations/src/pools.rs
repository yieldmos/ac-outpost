use cosmwasm_schema::cw_serde;
use cosmwasm_std::Storage;
use cw_storage_plus::Map;
use struct_iterable::Iterable;

#[cw_serde]
#[derive(Default, Iterable)]
pub struct OsmoPools {
    pub tia: OsmosisKnownPoolListing,
    pub ion: OsmosisKnownPoolListing,
    pub mars: OsmosisKnownPoolListing,
    pub usdc: OsmosisKnownPoolListing,
    pub atom: OsmosisKnownPoolListing,
    pub whale: OsmosisKnownPoolListing,
    pub mbrn: OsmosisKnownPoolListing,
    pub cdt: OsmosisKnownPoolListing,
}

#[cw_serde]
#[derive(Default, Iterable)]
pub struct UsdcPools {
    pub tia: OsmosisKnownPoolListing,
    pub atom: OsmosisKnownPoolListing,
    pub osmo: OsmosisKnownPoolListing,
    pub cdt: OsmosisKnownPoolListing,
    pub axlusdc: OsmosisKnownPoolListing,
}

// TODO: Consider deriving this from the other sources of pool/denom data
#[cw_serde]
#[derive(Default, Iterable)]
pub struct Denoms {
    pub usdc: String,
    pub axlusdc: String,
    pub osmo: String,
    pub ion: String,
    pub tia: String,
    pub atom: String,
    pub amp_osmo: String,
    pub mars: String,
    pub whale: String,
    pub amp_whale: String,
    pub mbrn: String,
    pub cdt: String,
}

#[cw_serde]
#[derive(Default)]
pub struct OsmosisKnownPoolListing {
    pub pool_id: u64,
    pub out_denom: String,
}

pub trait PoolForEach {
    fn pools(&self) -> Vec<OsmosisKnownPoolListing>;
    fn store_as_map(&self, storage: &mut dyn Storage, map: StoredPools) -> () {
        self.pools()
            .iter()
            .for_each(|pool: &OsmosisKnownPoolListing| {
                map.save(storage, &pool.out_denom, &pool.pool_id);
            });
    }
}

impl PoolForEach for OsmoPools {
    fn pools(&self) -> Vec<OsmosisKnownPoolListing> {
        self.iter()
            .filter_map(|(_, pool_listing)| pool_listing.downcast_ref::<OsmosisKnownPoolListing>())
            .map(|pool_listing| pool_listing.clone())
            .collect()
    }
}

impl PoolForEach for UsdcPools {
    fn pools(&self) -> Vec<OsmosisKnownPoolListing> {
        self.iter()
            .filter_map(|(_, pool_listing)| pool_listing.downcast_ref::<OsmosisKnownPoolListing>())
            .map(|pool_listing| pool_listing.clone())
            .collect()
    }
}

impl Denoms {
    pub fn denoms(&self) -> Vec<(&str, &String)> {
        self.iter()
            .filter_map(|(denom_short_name, denom)| {
                denom
                    .downcast_ref::<String>()
                    .map(|denom| (denom_short_name, denom))
            })
            .collect()
    }
}

// Map of denoms (e.g. ibc/1234) to their short name (e.g. "usdc")
pub type StoredDenoms<'a> = Map<'a, &'a str, String>;

// Map of denom (e.g. ibc/1234) to their pool id
pub type StoredPools<'a> = Map<'a, &'a str, u64>;

pub struct MultipleStoredPools<'a> {
    pub osmo: StoredPools<'a>,
    pub usdc: StoredPools<'a>,
}
