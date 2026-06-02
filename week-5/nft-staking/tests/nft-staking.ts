import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { NftStaking } from "../target/types/nft_staking";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { MPL_CORE_PROGRAM_ID } from "@metaplex-foundation/mpl-core";
import { assert } from "chai";
import {
  airdrop,
  getConfigPda,
  getRewardMintPda,
  getStakeInfoPda,
  getUpdateAuthorityPda,
  logTxSignature,
  logAccount,
  logStakeDetails,
  logConfigDetails,
  logSection,
  sleep,
} from "./helpers";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

describe("nft-staking", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.NftStaking as Program<NftStaking>;
  const connection = provider.connection;

  const admin = provider.wallet as anchor.Wallet;
  const user = Keypair.generate();

  const rewardsBps = 100;
  const freezePeriod = 0;

  let collectionKeypair: Keypair;
  let assetKeypair: Keypair;
  let configPda: PublicKey;
  let updateAuthorityPda: PublicKey;
  let rewardMintPda: PublicKey;
  let stakeInfoPda: PublicKey;

  before(async () => {
    logSection("Setup");

    await airdrop(connection, user.publicKey);
    logAccount("user (funded)", user.publicKey);
    logAccount("admin", admin.publicKey);

    collectionKeypair = Keypair.generate();
    assetKeypair = Keypair.generate();

    [configPda] = getConfigPda(program.programId);
    [updateAuthorityPda] = getUpdateAuthorityPda(
      collectionKeypair.publicKey,
      program.programId
    );
    [rewardMintPda] = getRewardMintPda(configPda, program.programId);
    [stakeInfoPda] = getStakeInfoPda(
      assetKeypair.publicKey,
      user.publicKey,
      program.programId
    );

    logAccount("collection", collectionKeypair.publicKey);
    logAccount("asset", assetKeypair.publicKey);
    logAccount("config PDA", configPda);
    logAccount("update authority PDA", updateAuthorityPda);
    logAccount("reward mint PDA", rewardMintPda);
    logAccount("stake info PDA", stakeInfoPda);

    try {
      const sig = await program.methods
        .initialize(rewardsBps, freezePeriod)
        .accountsPartial({
          admin: admin.publicKey,
          config: configPda,
          rewardMint: rewardMintPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      logTxSignature("initialize", sig);
    } catch {
      console.log("  [info] config already initialized, skipping");
    }
  });

  // Verify config PDA stores correct reward rate and freeze period
  it("initializes the program with config and reward mint", async () => {
    const config = await program.account.config.fetch(configPda);
    assert.equal(config.rewardsBps, rewardsBps, "rewards_bps should match");
    assert.equal(
      config.freezePeriod,
      freezePeriod,
      "freeze_period should match"
    );

    logConfigDetails({
      rewardsBps: config.rewardsBps,
      freezePeriod: config.freezePeriod,
      rewardMint: rewardMintPda,
      configPda,
    });
  });

  // Create an MPL Core collection with staked_count attribute
  it("creates a collection with Attributes plugin", async () => {
    const sig = await program.methods
      .createCollection(
        "Test Collection",
        "https://example.com/collection.json"
      )
      .accountsPartial({
        payer: admin.publicKey,
        collection: collectionKeypair.publicKey,
        config: configPda,
        updateAuthority: updateAuthorityPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin.payer, collectionKeypair])
      .rpc();

    logTxSignature("createCollection", sig);
    logAccount("collection", collectionKeypair.publicKey);
    logAccount("update authority", updateAuthorityPda);
  });

  // Mint an NFT asset into the collection with metadata attributes
  it("mints an asset into the collection", async () => {
    const sig = await program.methods
      .mintAsset("Test NFT #1", "https://example.com/nft1.json")
      .accountsPartial({
        user: user.publicKey,
        asset: assetKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        config: configPda,
        updateAuthority: updateAuthorityPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user, assetKeypair])
      .rpc();

    logTxSignature("mintAsset", sig);
    logAccount("asset", assetKeypair.publicKey);
    logAccount("owner", user.publicKey);
  });

  // Stake the asset: freeze it, create StakeInfo PDA, increment staked_count
  it("stakes the asset", async () => {
    const sig = await program.methods
      .stake()
      .accountsPartial({
        owner: user.publicKey,
        asset: assetKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        config: configPda,
        stakeInfo: stakeInfoPda,
        updateAuthority: updateAuthorityPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    logTxSignature("stake", sig);

    const stakeInfo = await program.account.stakeInfo.fetch(stakeInfoPda);
    logStakeDetails({
      owner: stakeInfo.owner,
      asset: stakeInfo.asset,
      collection: stakeInfo.collection,
      stakedAt: stakeInfo.stakedAt.toNumber(),
      lastClaimed: stakeInfo.lastClaimed.toNumber(),
    });

    assert.ok(
      stakeInfo.owner.equals(user.publicKey),
      "stake owner should match"
    );
    assert.ok(
      stakeInfo.asset.equals(assetKeypair.publicKey),
      "stake asset should match"
    );
    assert.ok(
      stakeInfo.collection.equals(collectionKeypair.publicKey),
      "stake collection should match"
    );
    assert.ok(stakeInfo.stakedAt.toNumber() > 0, "staked_at should be set");
  });

  // Unstake the asset: unfreeze, remove FreezeDelegate, close StakeInfo PDA
  it("unstakes the asset", async () => {
    const sig = await program.methods
      .unstake()
      .accountsPartial({
        owner: user.publicKey,
        asset: assetKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        config: configPda,
        stakeInfo: stakeInfoPda,
        updateAuthority: updateAuthorityPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    logTxSignature("unstake", sig);

    const stakeInfoAccount = await connection.getAccountInfo(stakeInfoPda);
    assert.isNull(
      stakeInfoAccount,
      "stake_info should be closed after unstake"
    );
    console.log("  [info] stake_info PDA closed, rent returned to owner");
  });

  // Verify that restaking works after a previous unstake cycle
  it("can restake after unstaking", async () => {
    // Wait for a new blockhash to avoid tx dedup rejection
    await sleep(500);

    const sig = await program.methods
      .stake()
      .accountsPartial({
        owner: user.publicKey,
        asset: assetKeypair.publicKey,
        collection: collectionKeypair.publicKey,
        config: configPda,
        stakeInfo: stakeInfoPda,
        updateAuthority: updateAuthorityPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    logTxSignature("restake", sig);

    const stakeInfo = await program.account.stakeInfo.fetch(stakeInfoPda);
    logStakeDetails({
      owner: stakeInfo.owner,
      asset: stakeInfo.asset,
      collection: stakeInfo.collection,
      stakedAt: stakeInfo.stakedAt.toNumber(),
      lastClaimed: stakeInfo.lastClaimed.toNumber(),
    });

    assert.ok(stakeInfo.stakedAt.toNumber() > 0, "restaked successfully");
  });
});
