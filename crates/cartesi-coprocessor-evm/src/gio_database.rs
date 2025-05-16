use alloy_rlp::Decodable;
use revm::{
    database_interface::async_db::DatabaseAsync,
    primitives::{Address, B256, U256},
    state::{AccountInfo, Bytecode},
};

use alloy_consensus::Header;
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
        let input = concat_bytes(&GIOHash::Keccak256.to_bytes(), &hash.to_vec());
        let response = self.client.emit_gio(GIODomain::GetImage, &input).await?;
        Ok(response.data)
    }

    async fn emit_hint(&self, hint: GIOHint, input: &Vec<u8>) -> Result<(), GIOError> {
        let input = concat_bytes(&hint.to_bytes(), input);
        self.client
            .emit_gio(GIODomain::PreimageHint, &input)
            .await?;
        Ok(())
    }

    async fn get_block_header(&self, block_hash: BlockHash) -> Result<Header, GIOError> {
        self.emit_hint(GIOHint::EthBlockPreimage, &block_hash.to_vec())
            .await?;
        
        let header_data = self.get_preimage(block_hash).await?;
        let mut header_data = header_data.as_slice();
        let header = Header::decode(&mut header_data)
            .map_err(|err| GIOError::BadResponseData(err.to_string()))?;
        Ok(header)
    }
}

impl DatabaseAsync for GIODatabase {
    type Error = GIOError;

    async fn basic_async(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        // Get account
        let input = concat_bytes(&self.block_hash.to_vec(), &address.to_vec());
        let response = self.client.emit_gio(GIODomain::GetAccount, &input).await?;

        let balance_data: [u8; 32] = response.data[0..32]
            .try_into()
            .expect("invalid account balance data length");
        let nonce_data: [u8; 8] = response.data[32..40]
            .try_into()
            .expect("invalid account nonce data length");
        let code_hash_data: [u8; 32] = response.data[40..72]
            .try_into()
            .expect("invalid account account code hash data length");

        // Get code
        let input = concat_bytes(&self.block_hash.to_vec(), &address.to_vec());
        self.emit_hint(GIOHint::EthCodePreimage, &input)
            .await?;
        let code_data = self.get_preimage(B256::from(code_hash_data)).await?;

        let account = AccountInfo {
            balance: U256::from_be_bytes(balance_data),
            nonce: u64::from_be_bytes(nonce_data),
            code_hash: B256::from(code_hash_data),
            code: Some(Bytecode::new_raw(Bytes::from(code_data))),
        };

        Ok(Some(account))
    }

    async fn code_by_hash_async(&mut self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        panic!("This should not be called, as the code is already loaded");
        // This is not needed, as the code is already loaded with basic_ref
    }

    async fn storage_async(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let input = concat_bytes(&self.block_hash.to_vec(), &address.to_vec());
        let input = concat_bytes(&input, &index.to_be_bytes_vec());
        let response = self.client.emit_gio(GIODomain::GetStorage, &input).await?;
        let slot_data: [u8; 32] = response
            .data
            .try_into()
            .expect("invalid storage slot data length");
        Ok(U256::from_be_bytes(slot_data))
    }

    async fn block_hash_async(&mut self, number: u64) -> Result<B256, Self::Error> {
        let mut block_hash = self.block_hash;
        loop {
            let header = self.get_block_header(block_hash).await?;
            if header.number == number {
                return Ok(header.hash_slow());
            }
            block_hash = header.parent_hash;
        }
    }
}

fn concat_bytes(v1: &Vec<u8>, v2: &Vec<u8>) -> Vec<u8> {
    [v1.as_slice(), v2.as_slice()].concat()
}
