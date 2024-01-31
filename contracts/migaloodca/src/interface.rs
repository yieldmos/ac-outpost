use cw_orch::{interface, prelude::*};

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg)]
pub struct YmosMigaloodcaOutpost;

impl<Chain: CwEnv> Uploadable for YmosMigaloodcaOutpost<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("ymos_migaloodca_outpost")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper(&self) -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(crate::contract::execute, crate::contract::instantiate, crate::contract::query)
                .with_migrate(crate::contract::migrate),
        )
    }
}
