use cosmwasm_schema::cw_serde;
use cw_storage_plus::Map;
use struct_iterable::Iterable;

#[cw_serde]
#[derive(Default)]
pub struct OsmosisKnownPoolListing {
    pub pool_id: u64,
    pub out_denom: String,
}

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

pub trait PoolForEach {
    fn for_each_pool(&self, each_fn: fn(&OsmosisKnownPoolListing) -> ()) -> ();
}

impl PoolForEach for OsmoPools {
    fn for_each_pool(&self, each_fn: fn(&OsmosisKnownPoolListing) -> ()) {
        self.iter().for_each(|(_, pool_listing)| {
            if let Some(listing) = pool_listing.downcast_ref::<OsmosisKnownPoolListing>() {
                each_fn(listing)
            }
        })
    }
}

impl PoolForEach for UsdcPools {
    fn for_each_pool(&self, each_fn: fn(&OsmosisKnownPoolListing) -> ()) {
        self.iter().for_each(|(_, pool_listing)| {
            if let Some(listing) = pool_listing.downcast_ref::<OsmosisKnownPoolListing>() {
                each_fn(listing)
            }
        })
    }
}

impl Denoms {
    fn for_each_denom(&self, each_fn: fn((&str, &str)) -> ()) -> () {
        self.iter().for_each(|(denom_short_name, denom)| {
            if let Some(denom) = denom.downcast_ref::<String>() {
                each_fn((denom_short_name, denom))
            }
        })
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
