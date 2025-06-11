# Video walkthrough

A video walkthrough of this component can be found here:

https://www.youtube.com/watch?v=ST9XS_N1Sbs

# General

Library `crates/cartesi-coprocessor-evm` allows to work with EVM instance within Cartesi virtual machine:

- `GIOClient` - implements Cartesi VM GIO client
- `GIODatabase` - implements `revm::DatabaseAsync` interface by querying GIO for current EVM state
- `EVM` - high-level wrapper around `revm` with support of issuing smart contract calls

Example usage - querying ERC20 token balance:

```rust
pub async fn query_erc20_balance(
    gio_client: GIOClient,
    block_hash: BlockHash,
    owner: Address,
    erc20: Address,
) -> Result<U256, EVMError> {
    sol! {
        function balanceOf(address) external view returns (uint256);
    }
    let encoded = balanceOfCall::new((owner,)).abi_encode();

    let mut evm = EVM::new(gio_client, block_hash);
    let result = evm.call(
        address!("0000000000000000000000000000000000000000"),
        erc20,
        0,
        U256::ZERO,
        Bytes::from(encoded),
    )?;

    let balance = balanceOfCall::abi_decode_returns(&result)
        .expect("failed to decode return value of balanceOf call");

    Ok(balance)
}
```
