use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_json_binary, Addr, Api, CosmosMsg, StdResult, Storage, Timestamp, Uint64, WasmMsg,
};

use crate::{
    msg::{CompPrefStatus, ExecuteMsg},
    state::{
        CompPref, EndType, InactiveStatus, UnverifiedUserCompPref, UserCompPref,
        ALLOWED_STRATEGY_IDS,
    },
    ContractError,
};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_json_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

impl CompPref {
    pub fn is_active(&self, current_timestamp: &Timestamp) -> bool {
        self.ended_timestamp(current_timestamp).is_none()
    }

    pub fn ended_timestamp(&self, current_timestamp: &Timestamp) -> Option<Timestamp> {
        self.cancelled_timestamp()
            .or_else(|| self.expired_timestamp(current_timestamp))
    }

    pub fn cancelled_timestamp(&self) -> Option<Timestamp> {
        if let Some(InactiveStatus {
            end_type: EndType::Cancellation,
            ended_at,
        }) = self.is_inactive
        {
            Some(ended_at)
        } else {
            None
        }
    }

    pub fn expired_timestamp(&self, current_timestamp: &Timestamp) -> Option<Timestamp> {
        match self.is_inactive {
            Some(InactiveStatus {
                end_type: EndType::Expiration,
                ended_at,
            }) => Some(ended_at),
            None if current_timestamp.gt(&self.user_comp_pref.expires) => {
                Some(self.user_comp_pref.expires)
            }
            _ => None,
        }
    }

    pub fn matches_status_filter(
        &self,
        status_filter: &Option<CompPrefStatus>,
        current_timestamp: &Timestamp,
    ) -> bool {
        match status_filter {
            None => true,
            Some(CompPrefStatus::Active) => self.is_active(current_timestamp),
            Some(CompPrefStatus::Inactive) => self.ended_timestamp(current_timestamp).is_some(),
            Some(CompPrefStatus::Expired) => self.expired_timestamp(current_timestamp).is_some(),
            Some(CompPrefStatus::Cancelled) => self.cancelled_timestamp().is_some(),
        }
    }
}

pub trait ValidStratId {
    fn valid_strat_id(&self, store: &dyn Storage) -> Result<(), ContractError>;
}

impl ValidStratId for Uint64 {
    fn valid_strat_id(&self, store: &dyn Storage) -> Result<(), ContractError> {
        if ALLOWED_STRATEGY_IDS.has(store, self.u64()) {
            Ok(())
        } else {
            Err(ContractError::InvalidStratId(*self))
        }
    }
}

impl UnverifiedUserCompPref {
    pub fn verify(
        &self,
        store: &dyn Storage,
        api: &dyn Api,
        current_time: &Timestamp,
    ) -> Result<UserCompPref, ContractError> {
        if self.expires.lt(current_time) {
            return Err(ContractError::InvalidExpiration(self.expires));
        }

        if self.strat_id.valid_strat_id(store).is_err() {
            return Err(ContractError::InvalidStratId(self.strat_id));
        }

        Ok(UserCompPref {
            outpost_address: api
                .addr_validate(&self.outpost_address)
                .map_err(|_| ContractError::InvalidOutpostAddress(self.outpost_address.clone()))?,
            address: api
                .addr_validate(&self.address)
                .map_err(|_| ContractError::InvalidUserAddress(self.address.clone()))?,
            strat_id: self.strat_id.u64(),
            strategy_settings: self.strategy_settings.clone(),
            comp_period: self.comp_period.clone(),
            pub_key: self.pub_key.clone(),
            expires: self.expires,
        })
    }
}

impl CompPref {
    pub fn new(
        chain_id: String,
        user_comp_pref: UserCompPref,
        current_time: Timestamp,
    ) -> CompPref {
        CompPref {
            chain_id,
            user_comp_pref,
            is_inactive: None,
            created_at: current_time,
            updated_at: current_time,
        }
    }
}
