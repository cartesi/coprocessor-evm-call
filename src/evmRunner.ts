import { Address } from '@ethereumjs/util'
import { VM } from '@ethereumjs/vm'
import { Common } from '@ethereumjs/common'
import { DefaultStateManager } from '@ethereumjs/statemanager'
import { CallParams, BlockchainAPI } from './types'
import { BlockHeader } from '@ethereumjs/block';

class CustomStateManager extends DefaultStateManager {
  private api: BlockchainAPI

  constructor(api: BlockchainAPI) {
    super()
    this.api = api
  }

  async getContractCode(address: Address): Promise<Buffer> {
    return this.api.getCode(address)
  }

  async getAccountData(address: Address) {
    const account = await this.api.getAccount(address)
    return {
      nonce: account.nonce,
      balance: account.balance,
      storageRoot: account.storageRoot,
      codeHash: account.codeHash,
    }
  }

  async getContractStorage(address: Address, key: Buffer): Promise<Buffer> {
    return this.api.getStorageSlot(address, key)
  }
  
}

export class EVMRunner {
  private api: BlockchainAPI
  private vm: VM

  private constructor(api: BlockchainAPI, vm: VM) {
    this.api = api
    this.vm = vm
  }

  static async create(api: BlockchainAPI, chain: string): Promise<EVMRunner> {
    const common = new Common({ chain: chain })
    const vm = await VM.create({
      common,
      stateManager: new CustomStateManager(api),
    })
    return new EVMRunner(api, vm)
  }

  async call(params: CallParams, blockHash: Buffer): Promise<Buffer> {
    const header = await this.api.getBlockHeader(blockHash);

    const result = await this.vm.evm.runCall({
      to: params.to,
      caller: params.from || Address.zero(),
      origin: params.from || Address.zero(),
      data: params.data,
      value: params.value || 0n,
      gasLimit: params.gas || header.gasLimit,
      block: { header }
    })

    if (result.execResult.exceptionError) {
      throw new Error(`Call failed: ${result.execResult.exceptionError.error}`)
    }

    return Buffer.from(result.execResult.returnValue)
  }
}

// Example usage:
/*
const api: BlockchainAPI = ... // Your API implementation
const runner = await EVMRunner.create(api)
const result = await runner.call({
  to: new Address(toBuffer('0x...')),
  data: toBuffer('0x...'),
  value: 0n,
})
console.log(result.toString('hex'))
*/ 