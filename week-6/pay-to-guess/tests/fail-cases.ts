import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { PayToGuess } from "../target/types/pay_to_guess";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  sendAndConfirmTransaction,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Transaction,
} from "@solana/web3.js";
import {
  buildPlayTx,
  expectAnchorError,
  fundAccount,
  getGamePDA,
  getPlayerStatePDA,
  getVaultPDA,
} from "./helper";
import dotenv from "dotenv";
import { expect } from "chai";

dotenv.config();

describe("pay-to-guess — fail cases", () => {
  const provider = anchor.AnchorProvider.env();
  provider.opts.commitment = "confirmed";
  provider.opts.preflightCommitment = "confirmed";
  anchor.setProvider(provider);
  const program = anchor.workspace.payToGuess as Program<PayToGuess>;
  const connection = provider.connection;

  const funder = (provider.wallet as anchor.Wallet).payer;

  const gameAuthority = Keypair.generate();
  const player = Keypair.generate();

  const PRIZE_POOL = new BN(0.005 * LAMPORTS_PER_SOL);
  const BET_BPS = 1000;

  const [gamePDA] = getGamePDA(gameAuthority.publicKey, program.programId);
  const [vaultPDA] = getVaultPDA(gamePDA, program.programId);
  const [psPDA] = getPlayerStatePDA(player.publicKey, program.programId);

  before(async () => {
    await fundAccount(connection, funder, gameAuthority.publicKey, 0.015);
    await fundAccount(connection, funder, player.publicKey, 0.005);

    await program.methods
      .initializeGame(PRIZE_POOL, BET_BPS)
      .accounts({
        authority: gameAuthority.publicKey,
        //@ts-ignore
        game: gamePDA,
        gameVault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([gameAuthority])
      .rpc();
  });

  // ── FAIL 1: play() without preceding payment ix ─────────────
  it("FAIL — play() without preceding payment instruction", async () => {
    await expectAnchorError(
      program.methods
        .play({ guess: 3 })
        .accounts({
          player: player.publicKey,
          //@ts-ignore
          playerState: psPDA,
          game: gamePDA,
          gameVault: vaultPDA,
          instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .signers([player])
        .rpc(),
      "NoRollAvailable"
    );
  });

  // ── FAIL 2: payment to wrong destination ────────────────────
  it("FAIL — payment to wrong destination", async () => {
    const state = await program.account.gameState.fetch(gamePDA);
    const betAmount = state.betAmount.toNumber();
    const badDest = Keypair.generate().publicKey;

    const tx = new Transaction();
    tx.add(
      SystemProgram.transfer({
        fromPubkey: player.publicKey,
        toPubkey: badDest,
        lamports: betAmount,
      })
    );
    tx.add(
      await program.methods
        .play({ guess: 2 })
        .accounts({
          player: player.publicKey,
          //@ts-ignore
          playerState: psPDA,
          game: gamePDA,
          gameVault: vaultPDA,
          instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    tx.feePayer = player.publicKey;
    tx.recentBlockhash = (
      await connection.getLatestBlockhash("confirmed")
    ).blockhash;

    await expectAnchorError(
      sendAndConfirmTransaction(connection, tx, [player], {
        skipPreflight: false,
      }),
      "NoRollAvailable"
    );
  });

  // ── FAIL 3: underpayment ────────────────────────────────────
  it("FAIL — underpayment", async () => {
    const state = await program.account.gameState.fetch(gamePDA);
    const half = Math.floor(state.betAmount.toNumber() / 2);

    const tx = new Transaction();
    tx.add(
      SystemProgram.transfer({
        fromPubkey: player.publicKey,
        toPubkey: vaultPDA,
        lamports: half,
      })
    );
    tx.add(
      await program.methods
        .play({ guess: 1 })
        .accounts({
          player: player.publicKey,
          //@ts-ignore
          playerState: psPDA,
          game: gamePDA,
          gameVault: vaultPDA,
          instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    tx.feePayer = player.publicKey;
    tx.recentBlockhash = (
      await connection.getLatestBlockhash("confirmed")
    ).blockhash;

    await expectAnchorError(
      sendAndConfirmTransaction(connection, tx, [player], {
        skipPreflight: false,
      }),
      "NoRollAvailable"
    );
  });

  // ── FAIL 4: guess out of range ──────────────────────────────
  it("FAIL — guess out of range", async () => {
    const state = await program.account.gameState.fetch(gamePDA);
    const betAmount = state.betAmount.toNumber();

    const tx = new Transaction();
    tx.add(
      SystemProgram.transfer({
        fromPubkey: player.publicKey,
        toPubkey: vaultPDA,
        lamports: betAmount,
      })
    );
    tx.add(
      await program.methods
        .play({ guess: 9 })
        .accounts({
          player: player.publicKey,
          //@ts-ignore
          playerState: psPDA,
          game: gamePDA,
          gameVault: vaultPDA,
          instructionSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    tx.feePayer = player.publicKey;
    tx.recentBlockhash = (
      await connection.getLatestBlockhash("confirmed")
    ).blockhash;

    await expectAnchorError(
      sendAndConfirmTransaction(connection, tx, [player], {
        skipPreflight: false,
      }),
      "InvalidGuess"
    );
  });

  // ── FAIL 5: play without roll ready ─────────────────────────
  it("FAIL — play without roll ready", async () => {
    const state = await program.account.gameState.fetch(gamePDA);
    expect(state.rollReady).to.equal(false);

    const tx = await buildPlayTx(
      program,
      player,
      gamePDA,
      vaultPDA,
      psPDA,
      state.betAmount.toNumber(),
      3
    );

    tx.feePayer = player.publicKey;
    tx.recentBlockhash = (
      await connection.getLatestBlockhash("confirmed")
    ).blockhash;

    await expectAnchorError(
      sendAndConfirmTransaction(connection, tx, [player], {
        skipPreflight: false,
      }),
      "NoRollAvailable"
    );
  });
});
