import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { DungeonVault } from "../target/types/dungeon_vault";
import { airdrop, getDungeonId } from "./helpers";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { getDungeonPDA, getPlayerStatePDA, getVaultPDA } from "./pdas";
import { assert } from "chai";

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

  let creator = provider.wallet;
  const player2 = Keypair.generate();
  const player3 = Keypair.generate();
  const player4 = Keypair.generate();

  const programId = program.programId;
  const connection = provider.connection;

  before(async () => {
    await airdrop(provider, creator.payer, player2.publicKey);
    await airdrop(provider, creator.payer, player3.publicKey);

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
});
