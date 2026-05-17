import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { DungeonVault } from "../target/types/dungeon_vault";
import {
  airdrop,
  DEFAULT_QUEUE,
  ER_URL,
  getDungeonFromER,
  getDungeonId,
  getErValidator,
  getPlayerStateFromER,
  requestVrfAndWait,
  submitChoiceOnER,
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
import { GetCommitmentSignature } from "@magicblock-labs/ephemeral-rollups-sdk";

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

  let winner: Keypair;
  let winnerStatePDA: PublicKey;

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
    console.log("erValidator:", erValidator.toBase58());

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
      // console.log(err.message);
      assert.include(err.message, "NotEnoughPlayers");
    }
  });

  // 2. Join Dungeon
  it("player1 joins and vault receives entry fee", async () => {
    const vaultBefore = await connection.getBalance(vaultPDA);

    console.log("vaultBefore", vaultBefore / LAMPORTS_PER_SOL);

    const joinDungeonIx = await program.methods
      .joinDungeon(dungeonId)
      .accounts({
        player: creator.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player1StatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    const delegatePlayerStateIx = await program.methods
      .delegateAccount({
        playerState: { dungeon: dungeonPDA, player: creator.publicKey },
      })
      .accounts({
        payer: creator.publicKey,
        //@ts-ignore
        pda: player1StatePDA,
        validator: erValidator,
      })
      .instruction();

    const tx = new Transaction().add(joinDungeonIx, delegatePlayerStateIx);

    tx.feePayer = creator.publicKey;

    const sig = await sendAndConfirmTransaction(
      provider.connection,
      tx,
      [creator.payer],
      { skipPreflight: true, commitment: "confirmed" }
    );

    console.log("player1 joined + delegated:", sig);

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

    const joinDungeonIx = await program.methods
      .joinDungeon(dungeonId)
      .accounts({
        player: player2.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player2StatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    const delegatePlayerStateIx = await program.methods
      .delegateAccount({
        playerState: { dungeon: dungeonPDA, player: player2.publicKey },
      })
      .accounts({
        payer: player2.publicKey,
        //@ts-ignore
        pda: player2StatePDA,
        validator: erValidator,
      })
      .instruction();

    const tx = new Transaction().add(joinDungeonIx, delegatePlayerStateIx);

    tx.feePayer = player2.publicKey;

    const sig = await sendAndConfirmTransaction(
      provider.connection,
      tx,
      [player2],
      { skipPreflight: true, commitment: "confirmed" }
    );

    console.log("player2 joined + delegated:", sig);

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

    const joinDungeonIx = await program.methods
      .joinDungeon(dungeonId)
      .accounts({
        player: player3.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: player3StatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    const delegatePlayerStateIx = await program.methods
      .delegateAccount({
        playerState: { dungeon: dungeonPDA, player: player3.publicKey },
      })
      .accounts({
        payer: player3.publicKey,
        //@ts-ignore
        pda: player3StatePDA,
        validator: erValidator,
      })
      .instruction();

    const tx = new Transaction().add(joinDungeonIx, delegatePlayerStateIx);
    tx.feePayer = player3.publicKey;

    const sig = await sendAndConfirmTransaction(
      provider.connection,
      tx,
      [player3],
      { skipPreflight: true, commitment: "confirmed" }
    );

    console.log("dungeonPDA delegated :", sig);

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

    console.log("dungeonPDA delegated :", sig);

    // After delegation, account owner changes to the delegation program
    const info = await connection.getAccountInfo(dungeonPDA);
    console.log("dungeonPDA owner after delegate:", info?.owner.toBase58());
    assert.ok(info, "dungeon account should still exist after delegation");
  });

  it("plays rounds until game ends", async function () {
    const players = [
      { name: "P1", keypair: creator.payer, pda: player1StatePDA },
      { name: "P2", keypair: player2, pda: player2StatePDA },
      { name: "P3", keypair: player3, pda: player3StatePDA },
    ];

    const allPDAs = players.map((p) => p.pda);

    for (let round = 1; round <= 15; round++) {
      const dungeon = await getDungeonFromER(
        ephemeralProgram.provider.connection,
        program,
        dungeonPDA
      );
      console.log(`\n========== ROUND ${round} ==========`);

      if (dungeon.status.finished || dungeon.alivePlayers <= 1) {
        console.log("Game finished");
        break;
      }

      console.log(
        `Alive: ${dungeon.alivePlayers} | Trap: ${dungeon.trapNumber}`
      );

      // submit choices
      console.log("\n-- Choices --");

      for (const p of players) {
        const ps = await getPlayerStateFromER(
          ephemeralProgram.provider.connection,
          program,
          p.pda
        ).catch(() => null);

        if (!ps?.alive) {
          console.log(`${p.name}: DEAD`);
          continue;
        }

        const pick = Math.floor(Math.random() * 3) + 1;

        const sig = await submitChoiceOnER(
          program,
          ephemeralProgram.provider.connection,
          dungeonId,
          dungeonPDA,
          p.pda,
          p.keypair,
          pick
        );

        console.log(`${p.name} -> ${pick} | ${sig}`);
      }

      // VRF
      console.log("\n-- VRF --");

      const vrfSig = await requestVrfAndWait(
        program,
        ephemeralProgram.provider.connection,
        dungeonId,
        dungeonPDA,
        creator.payer
      );

      console.log(`VRF sig: ${vrfSig}`);

      const vrfUpdatedDungeon = await getDungeonFromER(
        ephemeralProgram.provider.connection,
        program,
        dungeonPDA
      );

      console.log(`Trap selected by VRF: ${vrfUpdatedDungeon.trapNumber}`);
      // resolve
      console.log("\n-- Resolve --");

      const resolveIx = await program.methods
        .resolveRound(dungeonId)
        .accounts({
          authority: creator.publicKey,
          //@ts-ignore
          dungeon: dungeonPDA,
          vault: vaultPDA,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(
          allPDAs.map((pda) => ({
            pubkey: pda,
            isWritable: true,
            isSigner: false,
          }))
        )
        .instruction();

      const tx = new Transaction().add(resolveIx);
      tx.feePayer = creator.publicKey;

      const resolveSig = await sendAndConfirmTransaction(
        ephemeralProgram.provider.connection,
        tx,
        [creator.payer],
        {
          skipPreflight: true,
          commitment: "confirmed",
        }
      );

      console.log(`Resolve sig: ${resolveSig}`);

      // status
      console.log("\n-- Status --");

      const after = await getDungeonFromER(
        ephemeralProgram.provider.connection,
        program,
        dungeonPDA
      );

      for (const p of players) {
        const ps = await getPlayerStateFromER(
          ephemeralProgram.provider.connection,
          program,
          p.pda
        ).catch(() => null);

        console.log(`${p.name}: ${ps?.alive ? "ALIVE" : "DEAD"}`);
      }

      console.log(`Round: ${after.round} | Alive: ${after.alivePlayers}`);

      if (after.alivePlayers <= 1 || after.status.finished) {
        console.log("\n🏆 GAME OVER");
        break;
      }
    }

    const final = await getDungeonFromER(
      ephemeralProgram.provider.connection,
      program,
      dungeonPDA
    );

    console.log("\n========== FINAL ==========");
    console.log(`Alive Players: ${final.alivePlayers}`);
    console.log(`Round: ${final.round}`);
    console.log(`Trap Number: ${final.trapNumber}`);
  });

  it("undelegates dungeon + player accounts after game", async () => {
    const undelegateIx = await program.methods
      .undelegate(dungeonId)
      .accounts({
        player: creator.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
      })
      .remainingAccounts([
        {
          pubkey: player1StatePDA,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: player2StatePDA,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: player3StatePDA,
          isWritable: true,
          isSigner: false,
        },
      ])
      .instruction();

    const tx = new Transaction().add(undelegateIx);
    tx.feePayer = creator.publicKey;

    const sig = await sendAndConfirmTransaction(
      ephemeralProgram.provider.connection,
      tx,
      [creator.payer],
      {
        skipPreflight: true,
        commitment: "confirmed",
      }
    );

    console.log("Undelegate sig:", sig);

    const txCommitSgn = await GetCommitmentSignature(
      sig,
      ephemeralProgram.provider.connection
    );
    console.log("Undelegate commit sig:", txCommitSgn);

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);

    const p1 = await program.account.playerState.fetch(player1StatePDA);
    const p2 = await program.account.playerState.fetch(player2StatePDA);
    const p3 = await program.account.playerState.fetch(player3StatePDA);

    console.log("\nDungeon Status:", dungeon.status);
    console.log("Alive Players :", dungeon.alivePlayers);

    console.log("\nPlayer States");
    console.log("P1 alive:", p1.alive);
    console.log("P2 alive:", p2.alive);
    console.log("P3 alive:", p3.alive);
  });

  it("winner claims reward", async () => {
    const p1 = await program.account.playerState.fetch(player1StatePDA);
    const p2 = await program.account.playerState.fetch(player2StatePDA);
    const p3 = await program.account.playerState.fetch(player3StatePDA);

    if (p1.alive) {
      winner = creator.payer;
      winnerStatePDA = player1StatePDA;
    } else if (p2.alive) {
      winner = player2;
      winnerStatePDA = player2StatePDA;
    } else if (p3.alive) {
      winner = player3;
      winnerStatePDA = player3StatePDA;
    } else {
      winner = creator.payer;
      winnerStatePDA = player1StatePDA;

      console.log("Draw game -> creator withdraws vault");
    }

    console.log("Claimer:", winner.publicKey.toBase58());

    const beforeVault = await connection.getBalance(vaultPDA);
    const beforeWinner = await connection.getBalance(winner.publicKey);

    const claimTx = await program.methods
      .claimReward(dungeonId)
      .accounts({
        caller: winner.publicKey,
        //@ts-ignore
        dungeon: dungeonPDA,
        playerState: winnerStatePDA,
        vault: vaultPDA,
        systemProgram: SystemProgram.programId,
      })
      .signers([winner])
      .rpc();

    console.log("Claim sig:", claimTx);

    const afterVault = await connection.getBalance(vaultPDA);
    const afterWinner = await connection.getBalance(winner.publicKey);

    const dungeon = await program.account.dungeon.fetch(dungeonPDA);

    assert.equal(dungeon.claimed, true);
    assert.deepEqual(dungeon.status, { settled: {} });

    assert.isBelow(afterVault, beforeVault);
    assert.isAbove(afterWinner, beforeWinner);
  });
});
