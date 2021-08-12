import { PublicKey } from "@solana/web3.js";
import { createNewToken } from "./createToken";
import { payerAccount, connection } from "./init";
const DIVVY_PROGRAM_ID = new PublicKey("FatTSDYddftPGBVCoV6Uu2aCiMg8B8ZxV3QuoxE2PK6U")
const main1 = async () => {
    const [pda, bumpSeed] = await PublicKey.findProgramAddress([Buffer.from("divvyhouse")], DIVVY_PROGRAM_ID);
    const ht = await createNewToken(payerAccount, pda.toString(), pda.toString(), 6, connection);
    console.log("House token address:", ht);
    const usdt = await createNewToken(payerAccount, payerAccount.publicKey.toString(), payerAccount.publicKey.toString(), 6, connection);
    console.log("USDT token address:", usdt);
}
console.log("hello")
main1()