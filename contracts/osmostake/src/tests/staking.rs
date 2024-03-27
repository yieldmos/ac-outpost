use osmosis_std::types::cosmos::staking::v1beta1::{
    MsgCreateValidator, MsgCreateValidatorResponse, MsgDelegate, MsgDelegateResponse,
};

use cw_orch::osmosis_test_tube::{
    cosmrs::proto::cosmos::staking::v1beta1::{
        QueryDelegationRequest, QueryDelegationResponse, QueryValidatorRequest, QueryValidatorResponse,
        QueryValidatorsRequest, QueryValidatorsResponse,
    },
    osmosis_test_tube::{fn_execute, Module, Runner},
};

// Boilerplate code, copy and rename should just do the trick
pub struct Staking<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Staking<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}
// End Boilerplate code

impl<'a, R> Staking<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub delegate: MsgDelegate => MsgDelegateResponse
    }
    fn_execute! {
        pub create_validator: MsgCreateValidator => MsgCreateValidatorResponse
    }

    // fn_query! {
    //     pub query_delegation ["/cosmos.staking.v1beta1.Query/Delegation"]: QueryDelegationRequest => QueryDelegationResponse
    // }
    // fn_query! {
    //     pub query_validator ["/cosmos.staking.v1beta1.Query/Validator"]: QueryValidatorRequest => QueryValidatorResponse
    // }
    // fn_query! {
    //     pub query_validators ["/cosmos.staking.v1beta1.Query/Validators"]: QueryValidatorsRequest => QueryValidatorsResponse
    // }
}
