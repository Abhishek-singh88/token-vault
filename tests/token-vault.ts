import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenVault } from "../target/types/token_vault";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  getAccount,
  getMint,
} from "@solana/spl-token";

describe("Token Vault - Lock Repository 1 Token", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenVault as Program<TokenVault>;
  const owner = provider.wallet as anchor.Wallet;

  const YOUR_TOKEN_MINT = new PublicKey("BRxCtCD7SNPze2RqaTwZjG6YJKZBV9ZDWXihhUxQFfnE");

  let userTokenAccount: PublicKey;
  let vaultState: PublicKey;
  let vaultTokenAccount: PublicKey;

  before(async () => {
    console.log("Setting up existing vault...");
    console.log("Token Mint:", YOUR_TOKEN_MINT.toString());

    const tokenAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      owner.payer,
      YOUR_TOKEN_MINT,
      owner.publicKey,
      false,
      "confirmed",
      {},
      TOKEN_PROGRAM_ID
    );
    userTokenAccount = tokenAccountInfo.address;

    console.log("Token Account:", userTokenAccount.toString());

    const accountInfo = await getAccount(provider.connection, userTokenAccount);
    console.log("Token Balance:", accountInfo.amount.toString());
  });

  it("Deposit More Tokens to Existing Vault", async () => {
    console.log("Depositing to existing vault...");

    vaultState = new PublicKey("8ZqbuCG6PRDSN86mZW1iRNLKawQiPXqo43D6QyVvADyp");
    
    // Get vault info
    const vaultStateAccount = await program.account.vaultState.fetch(vaultState);
    vaultTokenAccount = vaultStateAccount.vaultTokenAccount;

    console.log("Existing Vault State:", vaultState.toString());
    console.log("Existing Vault Token Account:", vaultTokenAccount.toString());
    console.log("Current Locked Amount:", vaultStateAccount.amountLocked.toString());

    const beforeBalance = await getAccount(provider.connection, userTokenAccount);
    console.log("Balance before deposit:", beforeBalance.amount.toString());

    const mintInfo = await getMint(provider.connection, YOUR_TOKEN_MINT);
    const depositAmount = new anchor.BN(50000 * Math.pow(10, mintInfo.decimals)); // 50K tokens
    
    console.log("Depositing additional amount:", depositAmount.toString());

    const tx = await program.methods
      .depositTokens(depositAmount)
      .accounts({
        vaultState,
        vaultTokenAccount,
        userTokenAccount,
        tokenMint: YOUR_TOKEN_MINT,
        userAuthority: owner.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Additional Deposit Transaction:", tx);
    console.log("Explorer Link:", `https://explorer.solana.com/tx/${tx}?cluster=devnet`);

    const afterBalance = await getAccount(provider.connection, userTokenAccount);
    const vaultBalance = await getAccount(provider.connection, vaultTokenAccount);
    const updatedVaultState = await program.account.vaultState.fetch(vaultState);

    console.log("Balance after deposit:", afterBalance.amount.toString());
    console.log("Total vault balance:", vaultBalance.amount.toString());
    console.log("Total locked amount:", updatedVaultState.amountLocked.toString());

    console.log("=".repeat(60));
    console.log("REPOSITORY 2 COMPLETE - TOKENS LOCKED IN EXISTING VAULT");
    console.log("Vault State PDA:", vaultState.toString());
    console.log("Vault Token Account:", vaultTokenAccount.toString());
    console.log("Total Locked Amount:", updatedVaultState.amountLocked.toString());
    console.log("Additional Deposit TX:", tx);
    console.log("=".repeat(60));
  });
});
