use revm::{
    database_interface::async_db::DatabaseAsyncRef,
    primitives::{Address, B256, U256},
    state::{AccountInfo, Bytecode},
};

use alloy_primitives::{BlockHash, Bytes};

use crate::{
    gio_client::{GIOClient, GIODomain, GIOHash, GIOHint},
    gio_error::GIOError,
};

pub struct GIODatabase {
    client: GIOClient,
    block_hash: BlockHash,
}

impl GIODatabase {
    pub fn new(client: GIOClient, block_hash: BlockHash) -> Self {
        Self { client, block_hash }
    }

    async fn get_preimage(&self, hash: B256) -> Result<Vec<u8>, GIOError> {
        let data = concat_bytes(&GIOHash::Keccak256.to_bytes().to_vec(), &hash.to_vec());
        let response = self.client.emit_gio(GIODomain::GetImage, &data).await?;
        if !response.is_ok() {
            Err(GIOError::BadResponse {
                message: "failed to emit preimage".to_string(),
                response_code: response.code,
            })
        } else {
            Ok(response.data)
        }
    }

    async fn emit_hint(&self, hint: GIOHint, input: &Vec<u8>) -> Result<(), GIOError> {
        let data = concat_bytes(&hint.to_bytes().to_vec(), input);
        let response = self.client.emit_gio(GIODomain::PreimageHint, &data).await?;
        if !response.is_ok() {
            Err(GIOError::BadResponse {
                message: "failed to emit preimage".to_string(),
                response_code: response.code,
            })
        } else {
            Ok(())
        }
    }
}

fn concat_bytes(v1: &Vec<u8>, v2: &Vec<u8>) -> Vec<u8> {
    [v1.as_slice(), v2.as_slice()].concat()
}

impl DatabaseAsyncRef for GIODatabase {
    type Error = GIOError;

    async fn basic_async_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let data = concat_bytes(&self.block_hash.to_vec(), &address.to_vec());
        let response = self.client.emit_gio(GIODomain::PreimageHint, &data).await?;
        if !response.is_ok() {
            return Err(GIOError::BadResponse {
                message: "failed to get account".to_string(),
                response_code: response.code,
            });
        }

        let balance_data: [u8; 32] = response.data[0..32]
            .try_into()
            .expect("invalid account balance data length");
        let nonce_data: [u8; 8] = response.data[32..40]
            .try_into()
            .expect("invalid account nonce data length");
        let code_hash_data: [u8; 32] = response.data[40..72]
            .try_into()
            .expect("invalid account account code hash data length");

        self.emit_hint(GIOHint::EthCodePreimage, &address.to_vec())
            .await?;
        let code_data = self.get_preimage(B256::from(code_hash_data)).await?;

        let account = AccountInfo {
            balance: U256::from_le_bytes(balance_data),
            nonce: u64::from_le_bytes(nonce_data),
            code_hash: B256::from(code_hash_data),
            code: Some(Bytecode::new_raw(Bytes::from(code_data))),
        };

        Ok(Some(account))
    }

    async fn code_by_hash_async_ref(&self, _code_hash: B256) -> Result<Bytecode, Self::Error> {}

    async fn storage_async_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {}

    async fn block_hash_async_ref(&self, number: u64) -> Result<B256, Self::Error> {}
}
