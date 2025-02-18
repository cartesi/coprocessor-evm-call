import { Address } from '@ethereumjs/util';
import { BlockchainAPI } from './types';
import { BlockHeader } from '@ethereumjs/block';
import { Common } from '@ethereumjs/common';

interface GIOResponse {
    responseCode: number;
    response: Buffer;
}

interface GIOServerResponse {
    responseCode: number;
    response: string;
}

// GIO Domain constants
const GET_STORAGE_GIO = 0x27;
const GET_ACCOUNT_GIO = 0x29;
const GET_IMAGE_GIO = 0x2a;
const PREIMAGE_HINT_GIO = 0x2e;

// Hash types
const KECCAK256_HASH_TYPE = 2;

// Hint types
const HINT_ETH_CODE_PREIMAGE = 1;
const HINT_ETH_BLOCK_PREIMAGE = 2;

export async function emitGIO(
    serverAddr: string,
    domain: number,
    data: Buffer
): Promise<GIOResponse> {
    // Create GIO payload
    const hexData = '0x' + data.toString('hex');
    const gio = {
        domain,
        id: hexData
    };

    const response = await fetch(`${serverAddr}/gio`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(gio)
    });

    if (!response.ok) {
        throw new Error(`GIO request failed with status ${response.status}`);
    }

    const responseData = await response.json() as GIOServerResponse;
    // Remove '0x' prefix if present and convert to Buffer
    const cleanHex = responseData.response.startsWith('0x')
        ? responseData.response.slice(2)
        : responseData.response;

    return {
        responseCode: responseData.responseCode,
        response: Buffer.from(cleanHex, 'hex')
    };
}

export class GIOBlockchainAPI implements BlockchainAPI {
    private serverAddr: string;
    private latestBlockHash: Buffer;

    constructor(serverAddr: string, blockHash: Buffer) {
        this.serverAddr = serverAddr;
        this.latestBlockHash = blockHash;
    }

    private async getPreimage(hash: Buffer): Promise<Buffer> {
        // Construct input: hash_type (1) + hash (32)
        const input = Buffer.concat([
            Buffer.from([KECCAK256_HASH_TYPE]),
            hash
        ]);

        const response = await emitGIO(this.serverAddr, GET_IMAGE_GIO, input);
        if (response.responseCode !== 200) {
            throw new Error(`Failed to get preimage: ${response.responseCode}`);
        }
        return response.response;
    }

    private async emitHint(hintType: number, input: Buffer): Promise<void> {
        const response = await emitGIO(this.serverAddr, PREIMAGE_HINT_GIO, Buffer.concat([
            Buffer.from([hintType]),
            input
        ]));
        if (response.responseCode !== 200) {
            throw new Error(`Failed to emit hint: ${response.responseCode}`);
        }
    }

    async getStorageSlot(address: Address, slot: Buffer): Promise<Buffer> {
        // Construct input: blockHash (32) + address (20) + slot (32)
        const input = Buffer.concat([
            this.latestBlockHash,
            address.bytes,
            slot
        ]);

        const response = await emitGIO(this.serverAddr, GET_STORAGE_GIO, input);
        if (response.responseCode !== 200) {
            throw new Error(`Failed to get storage slot: ${response.responseCode}`);
        }
        return response.response;
    }

    async getAccount(address: Address): Promise<{
        nonce: bigint;
        balance: bigint;
        storageRoot: Buffer;
        codeHash: Buffer;
    }> {
        // Construct input: blockHash (32) + address (20)
        const input = Buffer.concat([
            this.latestBlockHash,
            address.bytes
        ]);

        const response = await emitGIO(this.serverAddr, GET_ACCOUNT_GIO, input);
        if (response.responseCode !== 200) {
            throw new Error(`Failed to get account: ${response.responseCode}`);
        }

        // Parse response:
        // balance (32) + nonce (8) + codeHash (32) + storageRoot (32)
        const responseData = response.response;
        
        return {
            balance: BigInt('0x' + responseData.subarray(0, 32).toString('hex')),
            nonce: BigInt('0x' + responseData.subarray(32, 40).toString('hex')),
            codeHash: responseData.subarray(40, 72),
            storageRoot: responseData.subarray(72, 104)
        };
    }

    async getCode(address: Address): Promise<Buffer> {
        // First emit hint for code preimage
        await this.emitHint(HINT_ETH_CODE_PREIMAGE, Buffer.concat([
            this.latestBlockHash,
            address.bytes
        ]));

        // Then get the account to get the codeHash
        const account = await this.getAccount(address);
        
        // If codeHash is empty (0x0), return empty buffer
        if (account.codeHash.equals(Buffer.alloc(32, 0))) {
            return Buffer.alloc(0);
        }

        // Get the code using the codeHash
        return this.getPreimage(account.codeHash);
    }

    getLatestBlockHash(): Promise<Buffer> {
        return Promise.resolve(this.latestBlockHash);
    }

    async getBlockHeader(blockHash: Buffer): Promise<BlockHeader> {
        // First emit hint for block header preimage
        await this.emitHint(HINT_ETH_BLOCK_PREIMAGE, blockHash);
        
        // Then get the block header data using the block hash
        const headerData = await this.getPreimage(blockHash);
        
        // Create header directly from RLP data
        const common = new Common({ chain: 'mainnet' });
        return BlockHeader.fromRLPSerializedHeader(headerData, { common });
    }
}