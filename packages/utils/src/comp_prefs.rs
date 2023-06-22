use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct CompoundPrefs<DestProjects> {
    pub relative: Vec<DestinationAction<DestProjects>>,
}

#[cw_serde]
pub struct DestinationAction<DestProjects> {
    pub destination: DestProjects,
    /// the percentage of the rewards that should be sent to this destination
    /// this is a number with 18 decimal places
    /// for example "250000000000000000" is 25%
    pub amount: u128,
}

#[cw_serde]
pub struct ValidatorSelection {
    pub validator_address: String,
    pub percent: Decimal,
}
