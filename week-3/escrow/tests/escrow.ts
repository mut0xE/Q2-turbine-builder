import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { Escrow } from "../target/types/escrow";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { createAccount, createMint, mintTo } from "@solana/spl-token";
import { assert } from "chai";

// constants
const ESCROW_SEED = Buffer.from("escrow");
const VAULT_SEED = Buffer.from("vault");
const USDC_DECIMALS = 6;

export async function airdrop(
  provider: anchor.Provider,
  from: Keypair,
  to: PublicKey
) {
  const fundTx = new Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: from.publicKey,
      toPubkey: to,
      lamports: 0.01 * LAMPORTS_PER_SOL,
    })
  );
  const fundSig = await provider.sendAndConfirm(fundTx, [from]);

  // console.log(`funded: ${fundSig}`);
}

describe("escrow", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.escrow as Program<Escrow>;
  const connection = provider.connection;

  const admin = provider.wallet as anchor.Wallet;
  const maker = Keypair.generate();
  const taker = Keypair.generate();

  // SPL token accounts
  let usdcMint: PublicKey;
  let makerUsdc: PublicKey;
  let takerUsdc: PublicKey;

  // PDAs
  let escrowPDA: PublicKey;
  let vaultPDA: PublicKey;

  // Escrow params
  const solAmount = new BN(0.001 * LAMPORTS_PER_SOL); // Maker locks 0.5 SOL
  const usdcAmount = new BN(10 * 10 ** USDC_DECIMALS); // Taker wants 10 USDC

  before(async () => {
    // Fund Taker
    await airdrop(provider, admin.payer, taker.publicKey);
    await airdrop(provider, admin.payer, maker.publicKey);

    // Create a USDC mint
    usdcMint = await createMint(
      connection,
      admin.payer, // payer
      admin.publicKey, // mint authority
      null, // freeze authority
      USDC_DECIMALS
    );
    console.log("USDC mint:", usdcMint.toBase58());

    // Create maker's USDC token account
    makerUsdc = await createAccount(
      connection,
      admin.payer,
      usdcMint,
      maker.publicKey
    );

    // Create taker's USDC token account + mint him 100 USDC
    takerUsdc = await createAccount(
      connection,
      admin.payer,
      usdcMint,
      taker.publicKey
    );
    await mintTo(
      connection,
      maker,
      usdcMint,
      takerUsdc,
      admin.payer,
      100 * 10 ** USDC_DECIMALS
    );

    // Derive PDAs
    [escrowPDA] = PublicKey.findProgramAddressSync(
      [ESCROW_SEED, maker.publicKey.toBuffer()],
      program.programId
    );
    [vaultPDA] = PublicKey.findProgramAddressSync(
      [VAULT_SEED, escrowPDA.toBuffer()],
      program.programId
    );

    console.log("escrowPDA :", escrowPDA.toBase58());
    console.log("vaultPDA  :", vaultPDA.toBase58());
    console.log("makerUsdc :", makerUsdc.toBase58());
    console.log("takerUsdc   :", takerUsdc.toBase58());
  });

  it("Maker creates escrow and locks SOL", async () => {
    const makerBefore = await connection.getBalance(maker.publicKey);

    await program.methods
      .make(solAmount, usdcAmount)
      .accounts({
        maker: maker.publicKey,
        //@ts-ignore
        escrow: escrowPDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    // Vault should hold exactly sol amount
    const vaultBal = await connection.getBalance(vaultPDA);
    assert.equal(vaultBal, solAmount.toNumber(), "vault holds SOL");

    // Escrow state correct
    const escrow = await program.account.escrow.fetch(escrowPDA);
    // console.log({ escrow });

    assert.equal(escrow.maker.toBase58(), maker.publicKey.toBase58());
    assert.equal(escrow.solAmount.toString(), solAmount.toString());
    assert.equal(escrow.usdcAmount.toString(), usdcAmount.toString());
    assert.deepEqual(escrow.status, { open: {} });

    const makerAfter = await connection.getBalance(maker.publicKey);
    console.log(
      `maker balance before: ${makerBefore / LAMPORTS_PER_SOL} and after: ${
        makerAfter / LAMPORTS_PER_SOL
      }`
    );
    console.log("Vault balance:", vaultBal / LAMPORTS_PER_SOL, "SOL");
  });

  it("rejects zero SOL amount", async () => {
    const testMaker = Keypair.generate();
    await airdrop(provider, admin.payer, testMaker.publicKey);

    const [pda] = PublicKey.findProgramAddressSync(
      [ESCROW_SEED, testMaker.publicKey.toBuffer()],
      program.programId
    );
    const [vault] = PublicKey.findProgramAddressSync(
      [VAULT_SEED, pda.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .make(new BN(0), usdcAmount)
        .accounts({
          maker: testMaker.publicKey,
          //@ts-ignore
          escrow: pda,
          vault,
          systemProgram: SystemProgram.programId,
        })
        .signers([testMaker])
        .rpc();
      assert.fail("should have thrown");
    } catch (e: any) {
      assert.include(e.message, "InvalidSolAmount");
    }
  });

  it("rejects zero USDC amount", async () => {
    const testMaker = Keypair.generate();
    await airdrop(provider, admin.payer, testMaker.publicKey);

    const [pda] = PublicKey.findProgramAddressSync(
      [ESCROW_SEED, testMaker.publicKey.toBuffer()],
      program.programId
    );
    const [vault] = PublicKey.findProgramAddressSync(
      [VAULT_SEED, pda.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .make(solAmount, new BN(0))
        .accounts({
          maker: testMaker.publicKey,
          //@ts-ignore
          escrow: pda,
          vault,
          systemProgram: SystemProgram.programId,
        })
        .signers([testMaker])
        .rpc();
      assert.fail("should have thrown");
    } catch (e: any) {
      assert.include(e.message, "InvalidUsdcAmount");
    }
  });
});
