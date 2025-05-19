use thiserror::Error;

use alloy_primitives::BlockHash;
use revm::{
    context::{
        result::{ExecutionResult, Output},
        Context,
    },
    database_interface::WrapDatabaseAsync,
    primitives::{Address, Bytes, TxKind, U256},
    ExecuteEvm, MainBuilder, MainContext,
};

use crate::{gio_client::GIOClient, gio_database::GIODatabase};

#[derive(Error, Debug)]
pub enum EVMError {
    #[error("evm execution failed: {0}")]
    ExecutionFailed(String),
}

pub struct EVM {
    database: WrapDatabaseAsync<GIODatabase>,
}

impl EVM {
    pub fn new(gio_client: GIOClient, block_hash: BlockHash) -> Self {
        let database = WrapDatabaseAsync::new(GIODatabase::new(gio_client, block_hash))
            .expect("failed to create evm database - no tokio runtime available");
        Self { database }
    }

    pub fn call(
        &mut self,
        caller: Address,
        to: Address,
        gas: u128,
        value: U256,
        data: Bytes,
    ) -> Result<Bytes, EVMError> {
        let mut evm = Context::mainnet()
            .with_db(&mut self.database)
            .modify_tx_chained(|tx| {
                tx.caller = caller;
                tx.kind = TxKind::Call(to);
                tx.data = data;
                tx.gas_price = gas;
                tx.value = value;
            })
            .build_mainnet();

        let ref_tx = evm
            .replay()
            .map_err(|err| EVMError::ExecutionFailed(err.to_string()))?;

        match ref_tx.result {
            ExecutionResult::Success {
                output: Output::Call(value),
                ..
            } => Ok(value),

            _ => {
                let msg = serde_json::to_string(&ref_tx.result).unwrap();
                Err(EVMError::ExecutionFailed(msg))
            }
        }
    }
}
