import { Address } from '@ethereumjs/util'
import { BlockHeader } from '@ethereumjs/block'

export interface BlockchainAPI {
  getAccount(address: Address): Promise<{
    nonce: bigint
    balance: bigint
    storageRoot: Buffer
    codeHash: Buffer
  }>
  getStorageSlot(address: Address, slot: Buffer): Promise<Buffer>
  getCode(address: Address): Promise<Buffer>
  getBlockHeader(blockHash: Buffer): Promise<BlockHeader>
}

export interface CallParams {
  to: Address
  from?: Address
  data: Buffer
  value?: bigint
  gas?: bigint
} 