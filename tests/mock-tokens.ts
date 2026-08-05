import * as anchor from "@coral-xyz/anchor";
import {
  createMint,
  createAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";

export async function createMockToken(
  provider: anchor.AnchorProvider,
  decimals: number,
  mintAuthority: anchor.web3.Keypair,
  isToken2022: boolean = false
): Promise<anchor.web3.PublicKey> {
  const programId = isToken2022 ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;

  const mint = await createMint(
    provider.connection,
    // Using a newly generated keypair for paying fees just for test isolation,
    // or we can use the provider's wallet by casting it
    (provider.wallet as any).payer, 
    mintAuthority.publicKey,
    null,
    decimals,
    undefined,
    undefined,
    programId
  );

  return mint;
}

export async function mintMockTokens(
  provider: anchor.AnchorProvider,
  mint: anchor.web3.PublicKey,
  mintAuthority: anchor.web3.Keypair,
  recipient: anchor.web3.PublicKey,
  amount: number,
  isToken2022: boolean = false
): Promise<anchor.web3.PublicKey> {
  const programId = isToken2022 ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;
  const payer = (provider.wallet as any).payer;

  // Create an ATA (or standard token account) for the recipient
  const tokenAccount = await createAccount(
    provider.connection,
    payer,
    mint,
    recipient,
    undefined,
    undefined,
    programId
  );

  // Mint tokens to the created account
  await mintTo(
    provider.connection,
    payer,
    mint,
    tokenAccount,
    mintAuthority,
    amount,
    [],
    undefined,
    programId
  );

  return tokenAccount;
}
