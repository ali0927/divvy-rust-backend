import { Account, Cluster, clusterApiUrl, Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction, TransactionInstruction } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
const { struct, nu64, u8, blob } = require("buffer-layout");
import fs from 'fs'
import path from "path";
import { createNewToken } from "./createToken";
import { createTokenAccount } from "./createTokenAccount";
const DIVVY_PROGRAM_ID = new PublicKey("6mevH4HoqvLVNUsnbn9dWg4iBt3EZMY6REcjasnQH1YE")
const payerAccount = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(path.join(process.env.HOME!, "/Desktop/divvy.json"), 'utf-8'))), { skipValidation: true })
const insuranceAccount = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(path.join(process.env.HOME!, "/Desktop/insurance.json"), 'utf-8'))), { skipValidation: true })
const profitsAccount = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(path.join(process.env.HOME!, "/Desktop/profits.json"), 'utf-8'))), { skipValidation: true })
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
    uint64("lockedLiquidity"),
    uint64("liveLiquidity"),
    uint64("bettorBalance"),
    uint64("pendingBets"),
    blob(32, "htMint"),
    blob(32, "poolUsdt"),
    blob(32, "insuranceFundUsdt"),
    blob(32, "divvyFoundationProceedsUsdt"),
    bool("frozenPool"),
    bool("frozenBetting")
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
    console.log(pda.toString())
    let cluster = 'devnet';
    let url = clusterApiUrl(toCluster(cluster), true);
    let connection = new Connection(url, 'processed');
    console.log("PDA", pda.toString())

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
    const ht = await createNewToken(payerAccount, pda.toString(), pda.toString(), 6, connection);
    console.log("House token address:", ht);
    const usdt = await createNewToken(payerAccount, payerAccount.publicKey.toString(), payerAccount.publicKey.toString(), 6, connection);
    console.log("USDT token address:", usdt);

    const insuranceUSDTAccount = await createTokenAccount(payerAccount, usdt, insuranceAccount.publicKey.toString(), connection)
    const profitsUSDTAccount = await createTokenAccount(payerAccount, usdt, profitsAccount.publicKey.toString(), connection)
    const ht_mint = new PublicKey(ht)
    const pool_usdt_account = await createTokenAccount(payerAccount, usdt, pda.toString(), connection)
    console.log("HP USDT ACCOUNT:", pool_usdt_account.toString());
    const dataBuffer = Buffer.alloc(INIT_PROGRAM_LAYOUT.span);
    INIT_PROGRAM_LAYOUT.encode(data, dataBuffer);
    const initProgramInstruction = new TransactionInstruction({
        keys: [
            { pubkey: payerAccount.publicKey, isSigner: true, isWritable: true },
            { pubkey: hp_state_account.publicKey, isSigner: false, isWritable: true },
            { pubkey: ht_mint, isSigner: false, isWritable: true },
            { pubkey: pool_usdt_account, isSigner: false, isWritable: true },
            { pubkey: insuranceUSDTAccount, isSigner: false, isWritable: true },
            { pubkey: profitsUSDTAccount, isSigner: false, isWritable: true },

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