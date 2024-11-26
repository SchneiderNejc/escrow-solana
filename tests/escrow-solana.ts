import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";
import { EscrowSolana } from "../target/types/escrow_solana";

// --------------------- Helper Functions ---------------------
// this airdrops sol to an address
async function airdropSol(publicKey, amount) {
  let airdropTx = await anchor
    .getProvider()
    .connection.requestAirdrop(
      publicKey,
      amount * anchor.web3.LAMPORTS_PER_SOL
    );
  await confirmTransaction(airdropTx);
}

async function confirmTransaction(tx) {
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
  // anchor.setProvider(provider);
  const program = anchor.workspace.EscrowSolana as Program<EscrowSolana>;

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
    await airdropSol(depositor.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
    // Airdrop SOL to the depositor and recipient for fees
    await provider.connection.requestAirdrop(depositor.publicKey, 1e9);
    await provider.connection.requestAirdrop(recipient.publicKey, 1e9);

    // Create a new mint
    mint = await createMint(
      provider.connection,
      depositor,
      depositor.publicKey,
      null,
      6, // Decimals
      undefined,
      undefined,
      TOKEN_PROGRAM_ID
    );

    // Create token accounts for depositor and recipient
    depositorTokenAccount = await createAccount(
      provider.connection,
      depositor,
      mint,
      depositor.publicKey
    );
    recipientTokenAccount = await createAccount(
      provider.connection,
      depositor,
      mint,
      recipient.publicKey
    );

    // Mint tokens to the depositor's token account
    await mintTo(
      provider.connection,
      depositor,
      mint,
      depositorTokenAccount,
      depositor.publicKey,
      1_000_000_000
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

    escrowTokenAccount = await Token.getAssociatedTokenAddress(
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
  });
});
