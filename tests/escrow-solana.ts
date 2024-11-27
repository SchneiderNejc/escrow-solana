import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  createMint,
  mintTo,
  createAccount,
} from "@solana/spl-token";
import { expect } from "chai";
import { EscrowSolana } from "../target/types/escrow_solana";
import { Idl } from "@coral-xyz/anchor";

// --------------------- Helper Functions ---------------------
// This airdrops SOL to an address and ensures the balance is updated
async function airdropSol(publicKey: PublicKey, amount: number) {
  console.log(`Requesting airdrop of ${amount} SOL to ${publicKey.toBase58()}`);

  const connection = anchor.getProvider().connection;

  // Request airdrop (convert SOL to lamports)
  const airdropTx = await connection.requestAirdrop(
    publicKey,
    amount * anchor.web3.LAMPORTS_PER_SOL
  );
  console.log(`Airdrop requested: ${airdropTx}`);

  // Confirm the transaction
  try {
    await confirmTransaction(airdropTx);
  } catch (error) {
    console.error("Error confirming airdrop transaction:", error);
  }

  // Check balance to verify airdrop success
  const balance = await connection.getBalance(publicKey);
  console.log(`Balance after airdrop: ${balance} lamports`);

  if (balance === 0) {
    throw new Error("Airdrop failed. Account has no funds.");
  }

  console.log(
    `Airdrop successful for ${publicKey.toBase58()}: ${balance} lamports`
  );
}

// Function to confirm the transaction
async function confirmTransaction(tx: string) {
  const latestBlockHash = await anchor
    .getProvider()
    .connection.getLatestBlockhash();

  await anchor.getProvider().connection.confirmTransaction({
    blockhash: latestBlockHash.blockhash,
    lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    signature: tx,
  });
}

// --------------------- Test Functions ---------------------

describe("escrow_solana", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  const program = anchor.workspace.EscrowSolana as Program<EscrowSolana & Idl>;

  // Keypairs and accounts
  const depositor = Keypair.generate();
  const recipient = Keypair.generate();
  let mint: PublicKey;
  let depositorTokenAccount: PublicKey;
  let recipientTokenAccount: PublicKey;
  let escrowAccount: PublicKey;
  let escrowTokenAccount: PublicKey;
  let escrowBump: number;

  before(async () => {
    // Airdrop SOL to the depositor and recipient for fees
    try {
      await airdropSol(depositor.publicKey, 2); // Airdrop 2 SOL to depositor
      await airdropSol(recipient.publicKey, 2); // Airdrop 2 SOL to recipient
    } catch (error) {
      console.log("Airdrop failed:", error);
    }

    // Create a new mint
    mint = await createMint(
      provider.connection,
      depositor,
      depositor.publicKey,
      null,
      9, // Decimals
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    const depositorTokenAccount = await createAccount(
      provider.connection,
      depositor, // Payer to create Token Account
      mint, // Mint Account address
      depositor.publicKey, // Token Account owner
      undefined, // Optional keypair, default to Associated Token Account
      undefined, // Confirmation options
      TOKEN_2022_PROGRAM_ID // Token Extension Program ID
    );

    const recipientTokenAccount = await createAccount(
      provider.connection,
      recipient, // Payer to create Token Account
      mint, // Mint Account address
      recipient.publicKey, // Token Account owner
      undefined, // Optional keypair, default to Associated Token Account
      undefined, // Confirmation options
      TOKEN_2022_PROGRAM_ID // Token Extension Program ID
    );

    // Mint tokens to the depositor's token account
    await mintTo(
      provider.connection,
      depositor,
      mint,
      depositorTokenAccount,
      depositor.publicKey,
      1_000_000_000, // Mint tokens to the depositor's account
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );
  });

  it("Creates an escrow", async () => {
    // Derive escrow and token accounts using findProgramAddressSync
    [escrowAccount, escrowBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), depositor.publicKey.toBuffer()],
      program.programId
    );

    const seeds = [Buffer.from("escrow")]; // Convert the string to a Buffer
    const [myPda, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      seeds,
      program.programId
    );

    escrowTokenAccount = await getAssociatedTokenAddress(
      TOKEN_PROGRAM_ID,
      mint,
      escrowAccount,
      program.programId
    );

    // Create escrow transaction
    const amount = 500_000_000; // 50 tokens (6 decimals)
    const expiry = 60; // 1 minute from now
    await program.methods
      .createEscrow(new anchor.BN(amount), new anchor.BN(expiry)) // Pass expiry as a BN
      .accounts({
        escrow: escrowAccount,
        depositor: depositor.publicKey,
        depositorTokenAccount,
        escrowTokenAccount,
        recipient: recipient.publicKey,
        mint,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([depositor])
      .rpc();

    const escrow = await program.account.escrow.fetch(escrowAccount);
    expect(escrow.depositor.toBase58()).to.equal(
      depositor.publicKey.toBase58()
    );
    expect(escrow.recipient.toBase58()).to.equal(
      recipient.publicKey.toBase58()
    );
    expect(escrow.amount.toNumber()).to.equal(amount);
    expect(escrow.status).to.equal(0); // Pending
  });

  it("Funds the escrow", async () => {
    // Fund the escrow
    await program.methods
      .fundEscrow()
      .accounts({
        escrow: escrowAccount,
        depositor: depositor.publicKey,
        depositorTokenAccount,
        escrowTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([depositor])
      .rpc();

    const escrow = await program.account.escrow.fetch(escrowAccount);
    expect(escrow.status).to.equal(0); // Still Pending

    // Check balances
    const escrowTokenBalance = await provider.connection.getTokenAccountBalance(
      escrowTokenAccount
    );
    expect(escrowTokenBalance.value.amount).to.equal("500000000");
  });

  xit("Withdraws from the escrow", async () => {
    // Wait for expiry to simulate an expired escrow
    await new Promise((resolve) => setTimeout(resolve, 60 * 1000));

    await program.methods
      .withdrawEscrow()
      .accounts({
        escrow: escrowAccount,
        recipient: recipient.publicKey,
        recipientTokenAccount,
        escrowTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([recipient])
      .rpc();

    const escrow = await program.account.escrow.fetch(escrowAccount);
    expect(escrow.status).to.equal(1); // Completed

    const recipientTokenBalance =
      await provider.connection.getTokenAccountBalance(recipientTokenAccount);
    expect(recipientTokenBalance.value.amount).to.equal("500000000");
  });
});
