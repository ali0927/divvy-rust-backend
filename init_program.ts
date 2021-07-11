import { Account, Cluster, clusterApiUrl, Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
const { struct, nu64, u8, blob } = require("buffer-layout");
import fs from 'fs'
import path from "path";
const DIVVY_PROGRAM_ID = new PublicKey("96qPDQTvLTQsNE9aQ73Xh3dRFj9UmX3Hpp48vpWuuTKj")
const payerAccount = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(path.join(process.env.HOME!, ".config/solana/id.json"), 'utf-8'))), { skipValidation: true })
//HPT Mint
// const mint = "FS3qbd4PQ4cvGWSMX9VSaQT7LwgEZwZcgiQpiRy3jkvV"
// const token_program = TOKEN_PROGRAM_ID
// const pda_account = "9tjYfuyjzs2ehSZvXAKa5wPYe2SCYpy9Q3nE57jry98Q"
// USDT account
// const hp_usdt_account = "F3hvLnCdPvmwjgEZ5LpYAZRaGB7GLU5DvM9wARkUNbjL"
const bool = (property = "bool") => {
    return blob(1, property);
};

/**
 * Layout for a 64bit unsigned value
 */
const uint64 = (property = "uint64") => {
    return blob(8, property);
};
export const STATE_ACCOUNT_DATA_LAYOUT = struct([
    bool("isInitialized"),
    uint64("availableLiquidity"),
	uint64("bettorBalance"),
	uint64("pendingBets"),
]);

const INIT_PROGRAM_LAYOUT = struct([
    u8("action"),
    u8("divvyPdaBumpSeed")
])

interface InitProgramData {
    action: number,
    divvyPdaBumpSeed: number
};

function toCluster(cluster: string): Cluster {
    switch (cluster) {
        case "devnet":
        case "testnet":
        case "mainnet-beta": {
            return cluster;
        }
    }
    throw new Error("Invalid cluster provided.");
}
const main = async () => {
    const [pda, bumpSeed] = await PublicKey.findProgramAddress([Buffer.from("divvyexchange")], DIVVY_PROGRAM_ID);

    let cluster = 'devnet';
    let url = clusterApiUrl(toCluster(cluster), true);
    let connection = new Connection(url, 'processed');


    const hp_state_account = Keypair.generate();
    console.log("Divvy HP state account " + hp_state_account.publicKey.toString())

    const data: InitProgramData = {
        action: 10,
        divvyPdaBumpSeed: bumpSeed
    };
    const create_hp_state = await SystemProgram.createAccount({
        space: STATE_ACCOUNT_DATA_LAYOUT.span,
        lamports: await connection.getMinimumBalanceForRentExemption(STATE_ACCOUNT_DATA_LAYOUT.span, 'singleGossip'),
        fromPubkey: payerAccount.publicKey,
        newAccountPubkey: hp_state_account.publicKey,
        programId: DIVVY_PROGRAM_ID
    });

    const dataBuffer = Buffer.alloc(INIT_PROGRAM_LAYOUT.span);

    INIT_PROGRAM_LAYOUT.encode(data, dataBuffer);

    const initProgramInstruction = new TransactionInstruction({
        keys: [
            { pubkey: payerAccount.publicKey, isSigner: true, isWritable: true },
            { pubkey: hp_state_account.publicKey, isSigner: false, isWritable: true },
        ],
        programId: DIVVY_PROGRAM_ID,
        data: dataBuffer,
    });

    console.log("Awaiting transaction confirmation...");
    
    let signature = await sendAndConfirmTransaction(connection, new Transaction().add(create_hp_state).add(initProgramInstruction), [
        payerAccount, hp_state_account
    ]);

    console.log(`https://explorer.solana.com/tx/${signature}?cluster=${cluster}`);
}
main()