import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { createMockToken, mintMockTokens } from "./mock-tokens";
import { assert } from "chai";

describe("stableperp", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace.Stableperp as any;

  // Keypairs for actors
  const admin = Keypair.generate();
  const writer = Keypair.generate();
  const buyer = Keypair.generate();
  const treasury = Keypair.generate();
  const buybackWallet = Keypair.generate();

  // Mints
  let underlyingMint: PublicKey;
  let quoteMint: PublicKey;
  
  // Market config
  const strike = new anchor.BN(100); // 1 AAPLx = 100 USDC (in smallest units for simplicity)
  let expiryTs: anchor.BN;
  const exerciseWindowSecs = new anchor.BN(60 * 60 * 24); // 24 hours

  // PDAs
  let configPda: PublicKey;
  let marketPda: PublicKey;
  let optionMintPda: PublicKey;
  let writerPositionPda: PublicKey;
  let escrowOptionVaultPda: PublicKey;
  
  before(async () => {
    // Airdrop SOL
    const sig1 = await provider.connection.requestAirdrop(admin.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    const sig2 = await provider.connection.requestAirdrop(writer.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    const sig3 = await provider.connection.requestAirdrop(buyer.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL);
    
    await provider.connection.confirmTransaction(sig1);
    await provider.connection.confirmTransaction(sig2);
    await provider.connection.confirmTransaction(sig3);

    // Create Mock Tokens
    underlyingMint = await createMockToken(provider, 6, admin);
    quoteMint = await createMockToken(provider, 6, admin);

    // Mint underlying (AAPLx) to writer
    await mintMockTokens(provider, underlyingMint, admin, writer.publicKey, 1000 * 10**6);
    // Mint quote (USDC) to buyer
    await mintMockTokens(provider, quoteMint, admin, buyer.publicKey, 10000 * 10**6);

    // Current time + some delay for expiry (e.g. past expiry to allow immediate exercise for testing)
    const now = Math.floor(Date.now() / 1000);
    expiryTs = new anchor.BN(now - 10); // Expiry in the past so we can exercise immediately
    
    [configPda] = PublicKey.findProgramAddressSync([Buffer.from("config")], program.programId);
    
    const strikeBytes = strike.toArrayLike(Buffer, "le", 8);
    const expiryBytes = expiryTs.toArrayLike(Buffer, "le", 8);
    [marketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), underlyingMint.toBuffer(), quoteMint.toBuffer(), strikeBytes, expiryBytes],
      program.programId
    );
    
    [optionMintPda] = PublicKey.findProgramAddressSync([Buffer.from("option_mint"), marketPda.toBuffer()], program.programId);
    
    [writerPositionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("writer"), marketPda.toBuffer(), writer.publicKey.toBuffer()],
      program.programId
    );
    
    [escrowOptionVaultPda] = PublicKey.findProgramAddressSync([Buffer.from("escrow"), writerPositionPda.toBuffer()], program.programId);
  });

  it("Initializes config", async () => {
    await program.methods
      .initConfig(100, treasury.publicKey, buybackWallet.publicKey)
      .accounts({
        config: configPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();
    
    const config = await program.account.config.fetch(configPda);
    assert.ok(config.admin.equals(admin.publicKey));
  });

  it("Initializes market", async () => {
    await program.methods
      .initMarket(strike, expiryTs, exerciseWindowSecs)
      .accounts({
        market: marketPda,
        config: configPda,
        admin: admin.publicKey,
        underlyingMint,
        quoteMint,
        optionMint: optionMintPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();
      
    const market = await program.account.market.fetch(marketPda);
    assert.ok(market.strike.eq(strike));
  });

  it("Writes an option", async () => {
    const qty = new anchor.BN(10 * 10**6); // 10 AAPLx
    const premiumAsk = new anchor.BN(5 * 10**6); // 5 USDC premium

    const writerUnderlyingAta = getAssociatedTokenAddressSync(underlyingMint, writer.publicKey);
    const collateralVault = getAssociatedTokenAddressSync(underlyingMint, marketPda, true);

    await program.methods
      .writeOption(qty, premiumAsk)
      .accounts({
        market: marketPda,
        writerPosition: writerPositionPda,
        collateralVault,
        writerUnderlyingAta,
        optionMint: optionMintPda,
        escrowOptionVault: escrowOptionVaultPda,
        writer: writer.publicKey,
        underlyingMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([writer])
      .rpc();
      
    const position = await program.account.writerPosition.fetch(writerPositionPda);
    assert.ok(position.mintedAmount.eq(qty));
  });

  it("Buys an option", async () => {
    const qtyToBuy = new anchor.BN(10 * 10**6); // 10 Options
    
    const buyerQuoteAta = getAssociatedTokenAddressSync(quoteMint, buyer.publicKey);
    const buyerOptionAta = getAssociatedTokenAddressSync(optionMintPda, buyer.publicKey);
    const writerQuoteAta = getAssociatedTokenAddressSync(quoteMint, writer.publicKey);

    // Create Writer Quote ATA if needed
    // Usually the buyer just transfers if it exists. But we need to make sure writer has quote ATA
    // For test, we mint 0 to create it or create manually
    await mintMockTokens(provider, quoteMint, admin, writer.publicKey, 0);

    await program.methods
      .buyOption(qtyToBuy)
      .accounts({
        market: marketPda,
        writerPosition: writerPositionPda,
        escrowOptionVault: escrowOptionVaultPda,
        writerQuoteAta,
        buyerOptionAta,
        buyerQuoteAta,
        optionMint: optionMintPda,
        quoteMint,
        buyer: buyer.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();
      
    const position = await program.account.writerPosition.fetch(writerPositionPda);
    assert.ok(position.filledAmount.eq(qtyToBuy));
  });

  it("Exercises the option", async () => {
    const qtyToExercise = new anchor.BN(5 * 10**6); // Exercise 5 Options

    const buyerOptionAta = getAssociatedTokenAddressSync(optionMintPda, buyer.publicKey);
    const buyerQuoteAta = getAssociatedTokenAddressSync(quoteMint, buyer.publicKey);
    const buyerUnderlyingAta = getAssociatedTokenAddressSync(underlyingMint, buyer.publicKey);
    
    const collateralVault = getAssociatedTokenAddressSync(underlyingMint, marketPda, true);
    const quoteVault = getAssociatedTokenAddressSync(quoteMint, marketPda, true);

    // Make sure buyer underlying ATA exists
    await mintMockTokens(provider, underlyingMint, admin, buyer.publicKey, 0);

    await program.methods
      .exerciseOption(qtyToExercise)
      .accounts({
        market: marketPda,
        collateralVault,
        quoteVault,
        exerciser: buyer.publicKey,
        exerciserOptionAta: buyerOptionAta,
        exerciserUnderlyingAta: buyerUnderlyingAta,
        exerciserQuoteAta: buyerQuoteAta,
        optionMint: optionMintPda,
        underlyingMint,
        quoteMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();
      
    // Verify buyer received underlying tokens
    const balance = await provider.connection.getTokenAccountBalance(buyerUnderlyingAta);
    assert.equal(balance.value.amount, qtyToExercise.toString());
  });
});
