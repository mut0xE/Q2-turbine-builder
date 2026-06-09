import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { PayToGuess } from "../target/types/pay_to_guess";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  buildPlayTx,
  calcBet,
  DEFAULT_QUEUE,
  ER_URL,
  fundAccount,
  getErValidator,
  getGamePDA,
  getPlayerStatePDA,
  getVaultPDA,
  logBalances,
} from "./helper";
import dotenv from "dotenv";
import { assert } from "chai";
import { randomBytes } from "crypto";

dotenv.config();

describe("pay-to-guess", () => {
  const provider = anchor.AnchorProvider.env();
  provider.opts.commitment = "confirmed";
  provider.opts.preflightCommitment = "confirmed";
  anchor.setProvider(provider);
  const program = anchor.workspace.payToGuess as Program<PayToGuess>;
  const connection = provider.connection;

  const providerER = new anchor.AnchorProvider(
    new anchor.web3.Connection(process.env.PROVIDER_ENDPOINT || ER_URL, {
      wsEndpoint: process.env.WS_ENDPOINT || "wss://devnet.magicblock.app/",
      commitment: "confirmed",
    }),
    anchor.Wallet.local()
  );
  const ephemeralProgram = new Program(program.idl, providerER);
  const erConnection = ephemeralProgram.provider.connection;

  const funder = (provider.wallet as anchor.Wallet).payer;
  const gameAuthority = Keypair.generate();
  const player = Keypair.generate();

  const PRIZE_POOL = new BN(0.005 * LAMPORTS_PER_SOL);
  const BET_BPS = 1000;

  const [gamePDA] = getGamePDA(gameAuthority.publicKey, program.programId);
  const [vaultPDA] = getVaultPDA(gamePDA, program.programId);
  const [playerPDA] = getPlayerStatePDA(player.publicKey, program.programId);

  const accts = () => [
    { label: "authority", pubkey: gameAuthority.publicKey },
    { label: "player", pubkey: player.publicKey },
    { label: "vault", pubkey: vaultPDA },
  ];

  // ── Helper: delegate -> VRF -> undelegate cycle ─────────────
  async function doVrfCycle(): Promise<void> {
    const callerSeed = randomBytes(1)[0];
    const erValidator = await getErValidator(ER_URL);

    const delegateIx = await program.methods
      .delegateAccount({ gameState: { authority: gameAuthority.publicKey } })
      .accounts({
        payer: gameAuthority.publicKey,
        //@ts-ignore
        pda: gamePDA,
        validator: erValidator,
      })
      .instruction();

    const delegateTx = new Transaction().add(delegateIx);
    delegateTx.feePayer = gameAuthority.publicKey;
    delegateTx.recentBlockhash = (
      await connection.getLatestBlockhash()
    ).blockhash;
    const delegateSig = await sendAndConfirmTransaction(
      connection,
      delegateTx,
      [gameAuthority],
      { skipPreflight: true, commitment: "confirmed" }
    );
    console.log("    delegate tx:", delegateSig);
    await new Promise((r) => setTimeout(r, 3000));

    let resolveCallback!: (sig: string) => void;
    const callbackPromise = new Promise<string>((r) => {
      resolveCallback = r;
    });
    const subId = erConnection.onLogs(
      program.programId,
      (info) => {
        if (
          !info.err &&
          info.logs.some((l) =>
            l.includes("Instruction: CallbackRandomness")
          ) &&
          info.logs.some((l) => l.includes("VRF roll"))
        )
          resolveCallback(info.signature);
      },
      "processed"
    );

    try {
      const vrfIx = await program.methods
        .requestRandomness(callerSeed)
        .accounts({
          payer: gameAuthority.publicKey,
          //@ts-ignore
          game: gamePDA,
          oracleQueue: DEFAULT_QUEUE,
        })
        .instruction();

      const vrfTx = new Transaction().add(vrfIx);
      vrfTx.feePayer = gameAuthority.publicKey;
      vrfTx.recentBlockhash = (
        await erConnection.getLatestBlockhash()
      ).blockhash;
      const vrfSig = await sendAndConfirmTransaction(
        erConnection,
        vrfTx,
        [gameAuthority],
        { skipPreflight: true, commitment: "confirmed" }
      );
      console.log("    VRF request tx:", vrfSig);

      console.log("    waiting for VRF callback...");
      const callbackSig = await Promise.race([
        callbackPromise,
        new Promise<null>((r) => setTimeout(() => r(null), 20_000)),
      ]);
      assert.ok(callbackSig, "VRF callback should arrive");
      console.log("    VRF callback tx:", callbackSig);
    } finally {
      await erConnection.removeOnLogsListener(subId);
    }

    const undelegateIx = await program.methods
      .undelegate()
      .accounts({
        payer: gameAuthority.publicKey,
        //@ts-ignore
        gameState: gamePDA,
      })
      .instruction();
    const undelegateTx = new Transaction().add(undelegateIx);
    undelegateTx.feePayer = gameAuthority.publicKey;
    undelegateTx.recentBlockhash = (
      await erConnection.getLatestBlockhash()
    ).blockhash;
    const undelegateSig = await sendAndConfirmTransaction(
      erConnection,
      undelegateTx,
      [gameAuthority],
      { skipPreflight: true, commitment: "confirmed" }
    );
    console.log("    undelegate tx:", undelegateSig);
    console.log("    waiting for devnet commit...");
    await new Promise((r) => setTimeout(r, 8000));
  }

  // ── Setup ───────────────────────────────────────────────────
  before(async () => {
    console.log("\n  Program       :", program.programId.toBase58());
    console.log("  Game Authority:", gameAuthority.publicKey.toBase58());
    console.log("  GamePDA       :", gamePDA.toBase58());
    console.log("  VaultPDA      :", vaultPDA.toBase58());
    console.log("  Player        :", player.publicKey.toBase58(), "\n");

    await fundAccount(connection, funder, gameAuthority.publicKey, 0.02);
    await fundAccount(connection, funder, player.publicKey, 0.01);
  });

  // ── 1. Initialize Game ──────────────────────────────────────
  it("1. initialize_game — vault created, SOL deposited", async () => {
    const sig = await program.methods
      .initializeGame(PRIZE_POOL, BET_BPS)
      .accounts({
        authority: gameAuthority.publicKey,
        //@ts-ignore
        game: gamePDA,
        gameVault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([gameAuthority])
      .rpc({ commitment: "confirmed" });
    console.log("  tx:", sig);

    const state = await program.account.gameState.fetch(gamePDA);
    assert.ok(state.prizePool.eq(PRIZE_POOL), "prize pool stored");
    assert.equal(state.betBps, BET_BPS, "bet bps stored");
    assert.ok(state.betAmount.toNumber() > 0, "bet amount calculated");
    assert.equal(state.totalRounds.toNumber(), 0);
    assert.equal(state.rollReady, false, "no roll yet");

    console.log("  Balances after init:");
    await logBalances(connection, accts());
  });

  // ── 2. Request Randomness ───────────────────────────────────
  it("2. request_randomness — delegate, VRF on ER, undelegate, verify devnet", async () => {
    await doVrfCycle();

    const state = await program.account.gameState.fetch(gamePDA);
    console.log("  devnet roll:", state.currentRoll);
    assert.equal(state.rollReady, true, "roll ready on devnet");
    assert.ok(state.currentRoll >= 1 && state.currentRoll <= 6);
  });

  // ── 3. Play ─────────────────────────────────────────────────
  it("3. play — introspection + settle win/lose", async () => {
    const state = await program.account.gameState.fetch(gamePDA);
    const betAmount = calcBet(state.prizePool.toNumber(), state.betBps);
    const roll = state.currentRoll;
    const guess = 3;

    console.log(
      "  guess:",
      guess,
      " roll:",
      roll,
      " bet:",
      betAmount / LAMPORTS_PER_SOL,
      "SOL"
    );
    console.log("  Balances before play:");
    const before = await logBalances(connection, accts());

    const tx = await buildPlayTx(
      program,
      player,
      gamePDA,
      vaultPDA,
      playerPDA,
      betAmount,
      guess
    );
    tx.feePayer = player.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

    const sig = await sendAndConfirmTransaction(connection, tx, [player], {
      skipPreflight: false,
      commitment: "confirmed",
    });
    console.log("  play tx:", sig);

    // Verify PlayerState
    const ps = await program.account.playerState.fetch(playerPDA);
    assert.equal(ps.currentGuess, guess, "current guess stored");
    assert.equal(ps.previousGuess, 0, "previous guess 0 on first play");
    assert.ok(ps.currentPaid.toNumber() > 0, "paid stored");
    assert.equal(ps.totalRounds.toNumber(), 1, "round counted");
    assert.deepEqual(ps.player, player.publicKey, "player stored");

    // Verify GameState
    const gameAfter = await program.account.gameState.fetch(gamePDA);
    assert.equal(gameAfter.rollReady, false, "roll consumed");
    assert.equal(gameAfter.currentRoll, 0, "roll cleared");
    assert.equal(gameAfter.totalRounds.toNumber(), 1, "round incremented");

    // Verify balances
    console.log("  Balances after play:");
    const after = await logBalances(connection, accts());
    const won = guess === roll;

    if (won) {
      assert.ok(
        after.get("player")! > before.get("player")!,
        "player gained SOL"
      );
      assert.ok(after.get("vault")! < before.get("vault")!, "vault shrank");
      console.log("  Result: WIN");
    } else {
      assert.ok(after.get("vault")! > before.get("vault")!, "vault grew");
      console.log("  Result: LOSE");
    }
  });

  // ── 4. Second play round ────────────────────────────────────
  it("4. second play round — VRF + play + verify PlayerState history", async () => {
    await doVrfCycle();

    const state = await program.account.gameState.fetch(gamePDA);
    const betAmount = calcBet(state.prizePool.toNumber(), state.betBps);
    const roll = state.currentRoll;
    const guess = 5;

    console.log(
      "  guess:",
      guess,
      " roll:",
      roll,
      " bet:",
      betAmount / LAMPORTS_PER_SOL,
      "SOL"
    );
    console.log("  Balances before play:");
    await logBalances(connection, accts());

    const tx = await buildPlayTx(
      program,
      player,
      gamePDA,
      vaultPDA,
      playerPDA,
      betAmount,
      guess
    );
    tx.feePayer = player.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

    const sig = await sendAndConfirmTransaction(connection, tx, [player], {
      skipPreflight: false,
      commitment: "confirmed",
    });
    console.log("  play tx:", sig);

    // Verify PlayerState updated for round 2
    const ps = await program.account.playerState.fetch(playerPDA);
    assert.equal(ps.currentGuess, guess, "current guess = 5");
    assert.equal(ps.previousGuess, 3, "previous guess = 3 (from test 3)");
    assert.equal(ps.totalRounds.toNumber(), 2, "2 rounds played");

    console.log("  Balances after play:");
    await logBalances(connection, accts());
  });

  // ── 5. Close Game ───────────────────────────────────────────
  it("5. close_game — authority reclaims all lamports", async () => {
    console.log("  Balances before close:");
    const before = await logBalances(connection, accts());

    const sig = await program.methods
      .closeGame()
      .accounts({
        authority: gameAuthority.publicKey,
        //@ts-ignore
        game: gamePDA,
        gameVault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([gameAuthority])
      .rpc({ commitment: "confirmed" });
    console.log("  close tx:", sig);

    console.log("  Balances after close:");
    const after = await logBalances(connection, accts());

    assert.ok(
      after.get("authority")! > before.get("authority")!,
      "authority reclaimed SOL"
    );
    assert.equal(after.get("vault")!, 0, "vault drained");

    const gameClosed = await connection.getAccountInfo(gamePDA);
    assert.isNull(gameClosed, "game PDA closed");
  });
});
