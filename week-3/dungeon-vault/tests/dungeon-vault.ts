import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { DungeonVault } from "../target/types/dungeon_vault";
import {
  airdrop,
  DEFAULT_QUEUE,
  ER_URL,
  getDungeonId,
  getErValidator,
} from "./helpers";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { getDungeonPDA, getPlayerStatePDA, getVaultPDA } from "./pdas";
import { assert } from "chai";
import { randomBytes } from "crypto";
import { waitUntilPermissionActive } from "@magicblock-labs/ephemeral-rollups-sdk";

describe("dungeon-vault", () => {
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.dungeonVault as Program<DungeonVault>;

  const dungeonId = getDungeonId();
  const entryFee = new BN(0.003 * LAMPORTS_PER_SOL);
  const maxPlayers = 3;

  let dungeonPDA: PublicKey;
  let vaultPDA: PublicKey;
  let player1StatePDA: PublicKey;
  let player2StatePDA: PublicKey;
  let player3StatePDA: PublicKey;
  let erValidator: PublicKey;

  let creator = provider.wallet;
  const player2 = Keypair.generate();
  const player3 = Keypair.generate();
  const player4 = Keypair.generate();

  const programId = program.programId;
  const connection = provider.connection;

  const providerEphemeralRollup = new anchor.AnchorProvider(
    new anchor.web3.Connection(process.env.PROVIDER_ENDPOINT || ER_URL, {
      wsEndpoint: process.env.WS_ENDPOINT || "wss://devnet.magicblock.app/",
    }),
    anchor.Wallet.local()
  );
  const ephemeralProgram = new Program(program.idl, providerEphemeralRollup);

  before(async () => {
    await airdrop(provider, creator.payer, player2.publicKey);
    await airdrop(provider, creator.payer, player3.publicKey);

    erValidator = await getErValidator(ER_URL);
    console.log("erValidator", erValidator);
    [dungeonPDA] = getDungeonPDA(dungeonId, creator.publicKey, programId);
    [vaultPDA] = getVaultPDA(dungeonPDA, programId);

    [player1StatePDA] = getPlayerStatePDA(
      dungeonPDA,
      creator.publicKey,
      programId
    );

    [player2StatePDA] = getPlayerStatePDA(
      dungeonPDA,
      player2.publicKey,
      programId
    );

    [player3StatePDA] = getPlayerStatePDA(
      dungeonPDA,
      player3.publicKey,
      programId
    );

    console.log("dungeonId    :", dungeonId.toString());
    console.log("dungeonPDA   :", dungeonPDA.toBase58());
    console.log("vaultPDA     :", vaultPDA.toBase58());
    console.log("player1State :", player1StatePDA.toBase58());
    console.log("player2State :", player2StatePDA.toBase58());
    console.log("player3State :", player3StatePDA.toBase58());
    console.log("creator      :", creator.publicKey.toBase58());
    console.log("player2      :", player2.publicKey.toBase58());
    console.log("player3      :", player3.publicKey.toBase58());
  });

  // 1. Initialize Dungeon
  it("creates dungeon with correct state", async () => {
    await program.methods
      .initializeDungeon(dungeonId, entryFee, maxPlayers)
      .accounts({
        creator: creator.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([creator.payer])
      .rpc();

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);
    assert.equal(dungeon.authority.toBase58(), creator.publicKey.toBase58());
    assert.equal(dungeon.entryFee.toString(), entryFee.toString());
    assert.equal(dungeon.maxPlayers, maxPlayers);
    assert.equal(dungeon.totalPlayers, 0);
    assert.equal(dungeon.alivePlayers, 0);
    assert.deepEqual(dungeon.status, { waiting: {} });
    assert.equal(dungeon.claimed, false);
  });

  it("fails with zero entry fee", async () => {
    const id = new BN(99);
    const [pda] = getDungeonPDA(id, creator.publicKey, programId);
    const [vault] = getVaultPDA(pda, programId);

    try {
      await program.methods
        .initializeDungeon(id, new BN(0), maxPlayers)
        .accounts({
          creator: creator.publicKey,
          //@ts-ignore
          dungeon: pda,
          vault: vault,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator.payer])
        .rpc();
      assert.fail("Should have thrown");
    } catch (err: any) {
      assert.include(err.message, "InvalidEntryFee");
    }
  });

  it("fails with invalid max players (1 player)", async () => {
    const id = getDungeonId();
    const [pda] = getDungeonPDA(id, creator.publicKey, programId);
    const [vault] = getVaultPDA(pda, programId);

    try {
      await program.methods
        .initializeDungeon(id, entryFee, 1)
        .accounts({
          creator: creator.publicKey,
          //@ts-ignore
          dungeon: pda,
          vault: vault,
          systemProgram: SystemProgram.programId,
        })
        .signers([creator.payer])
        .rpc();
      assert.fail("Should have thrown");
    } catch (err: any) {
      console.log(err.message);
      assert.include(err.message, "NotEnoughPlayers");
    }
  });

  // 2. Join Dungeon
  it("player1 joins and vault receives entry fee", async () => {
    const vaultBefore = await connection.getBalance(vaultPDA);
    console.log("vaultBefore", vaultBefore / LAMPORTS_PER_SOL);

    await program.methods
      .joinDungeon(dungeonId)
      .accounts({
        player: creator.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player1StatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([creator.payer])
      .rpc();

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);
    const playerState = await program.account.playerState.fetch(
      player1StatePDA
    );
    const vaultAfter = await connection.getBalance(vaultPDA);
    console.log("vaultAfter", vaultAfter / LAMPORTS_PER_SOL);

    assert.equal(playerState.player.toBase58(), creator.publicKey.toBase58());
    assert.equal(playerState.alive, true);
    assert.equal(playerState.currentChoice, 0);
    assert.isAbove(vaultAfter, vaultBefore);
    assert.equal(dungeon.amount.toString(), entryFee.toString());
  });

  it("player2 joins", async () => {
    const vaultBefore = await connection.getBalance(vaultPDA);
    console.log("vaultBefore", vaultBefore / LAMPORTS_PER_SOL);

    await program.methods
      .joinDungeon(dungeonId)
      .accounts({
        player: player2.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player2StatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([player2])
      .rpc();

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);
    const playerState = await program.account.playerState.fetch(
      player2StatePDA
    );
    const vaultAfter = await connection.getBalance(vaultPDA);
    console.log("vaultAfter", vaultAfter / LAMPORTS_PER_SOL);

    assert.equal(playerState.player.toBase58(), player2.publicKey.toBase58());
    assert.equal(playerState.alive, true);
    assert.equal(playerState.currentChoice, 0);
    assert.isAbove(vaultAfter, vaultBefore);

    assert.equal(dungeon.amount.toString(), entryFee.muln(2).toString());
    assert.equal(dungeon.alivePlayers, 2);
  });

  it("player3 joins and game becomes Active", async () => {
    const vaultBefore = await connection.getBalance(vaultPDA);
    console.log("vaultBefore", vaultBefore / LAMPORTS_PER_SOL);

    await program.methods
      .joinDungeon(dungeonId)
      .accounts({
        player: player3.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player3StatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([player3])
      .rpc();

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);
    const playerState = await program.account.playerState.fetch(
      player3StatePDA
    );
    const vaultAfter = await connection.getBalance(vaultPDA);
    console.log("vaultAfter", vaultAfter / LAMPORTS_PER_SOL);

    assert.equal(playerState.player.toBase58(), player3.publicKey.toBase58());
    assert.equal(playerState.alive, true);
    assert.equal(playerState.currentChoice, 0);
    assert.isAbove(vaultAfter, vaultBefore);

    assert.equal(dungeon.amount.toString(), entryFee.muln(3).toString());
    assert.equal(dungeon.alivePlayers, 3);
    assert.deepEqual(dungeon.status, { active: {} }); // game started
  });

  it("fails when dungeon is full (already Active)", async () => {
    await airdrop(provider, creator.payer, player4.publicKey);
    const [p4StatePDA] = getPlayerStatePDA(
      dungeonPDA,
      player4.publicKey,
      programId
    );

    try {
      await program.methods
        .joinDungeon(dungeonId)
        .accounts({
          player: player4.publicKey,
          //@ts-ignore
          dungeon: dungeonPDA,
          playerState: p4StatePDA,
          vault: vaultPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([player4])
        .rpc();
      assert.fail("Should have thrown");
    } catch (err: any) {
      assert.include(err.message, "GameAlreadyStarted");
    }
  });

  it("delegates dungeon PDA to ephemeral rollup", async () => {
    const delegateDungeonIx = await program.methods
      .delegateAccount({
        dungeon: { dungeonId, creator: creator.publicKey },
      })
      .accounts({
        payer: creator.publicKey,
        //@ts-ignore
        pda: dungeonPDA,
        validator: erValidator,
      })
      .instruction();

    const tx = new Transaction().add(delegateDungeonIx);
    tx.feePayer = creator.publicKey;

    const sig = await sendAndConfirmTransaction(
      provider.connection,
      tx,
      [creator.payer],
      { skipPreflight: true, commitment: "confirmed" }
    );

    const active = await waitUntilPermissionActive(ER_URL, dungeonPDA);
    console.log("dungeonPDA delegated :", sig);

    // After delegation, account owner changes to the delegation program
    const info = await connection.getAccountInfo(dungeonPDA);
    console.log("dungeonPDA owner after delegate:", info?.owner.toBase58());
    assert.ok(info, "dungeon account should still exist after delegation");
  });

  it("player1 submits a valid choice", async () => {
    await program.methods
      .submitChoice(dungeonId, 1)
      .accounts({
        player: creator.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player1StatePDA,
      })
      .signers([creator.payer])
      .rpc();

    const ps = await program.account.playerState.fetch(player1StatePDA);
    console.log("ps", ps);
    assert.equal(ps.currentChoice, 1);
  });

  it("player2 submits a valid choice", async () => {
    await program.methods
      .submitChoice(dungeonId, 2)
      .accounts({
        player: player2.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player2StatePDA,
      })
      .signers([player2])
      .rpc();

    const ps = await program.account.playerState.fetch(player2StatePDA);
    console.log("ps", ps);
    assert.equal(ps.currentChoice, 2);
  });

  it("player3 submits a valid choice", async () => {
    await program.methods
      .submitChoice(dungeonId, 3)
      .accounts({
        player: player3.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player3StatePDA,
      })
      .signers([player3])
      .rpc();

    const ps = await program.account.playerState.fetch(player3StatePDA);
    console.log("ps", ps);
    assert.equal(ps.currentChoice, 3);
  });

  it("fails with choice out of range (0)", async () => {
    try {
      await program.methods
        .submitChoice(dungeonId, 0)
        .accounts({
          player: creator.publicKey,
          //@ts-ignore
          dungeon: dungeonPDA,
          playerState: player1StatePDA,
        })
        .signers([creator.payer])
        .rpc();
      assert.fail("Should have thrown");
    } catch (err: any) {
      // console.log(err.message);
      assert.include(err.message, "InvalidChoice");
    }
  });

  it("fails with choice out of range (4)", async () => {
    try {
      await program.methods
        .submitChoice(dungeonId, 4)
        .accounts({
          player: player2.publicKey,
          //@ts-ignore
          dungeon: dungeonPDA,
          playerState: player2StatePDA,
        })
        .signers([player2])
        .rpc();
      assert.fail("Should have thrown");
    } catch (err: any) {
      // console.log(err.message);
      assert.include(err.message, "InvalidChoice");
    }
  });

  it("requests randomness via MagicBlock VRF and waits for callback", async () => {
    const callerSeed = randomBytes(1)[0];
    console.log("callerSeed:", callerSeed);

    const requestIx = await program.methods
      .requestRandomness(dungeonId, callerSeed)
      .accounts({
        payer: creator.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        oracleQueue: DEFAULT_QUEUE,
      })
      .instruction();
    const tx = new Transaction().add(requestIx);
    tx.feePayer = creator.publicKey;
    tx.recentBlockhash = (
      await ephemeralProgram.provider.connection.getLatestBlockhash()
    ).blockhash;

    const sig = await sendAndConfirmTransaction(
      ephemeralProgram.provider.connection,
      tx,
      [creator.payer],
      { skipPreflight: true, commitment: "confirmed" }
    );
    console.log("VRF request signature:", sig);

    console.log("Waiting for VRF callback...");
    await new Promise((resolve) => setTimeout(resolve, 10000));

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);
    console.log("Dungeon", dungeon);
    console.log("\nDungeon after VRF callback:");
    console.log("  trapNumber  :", dungeon.trapNumber);
    console.log("  round       :", dungeon.round);
    console.log("  alivePlayers:", dungeon.alivePlayers);
  });
});
