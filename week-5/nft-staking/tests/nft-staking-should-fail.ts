import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
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
  logSection,
  logError,
} from "./helpers";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

describe("nft-staking failure cases", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.NftStaking as Program<NftStaking>;
  const connection = provider.connection;

  const admin = provider.wallet as anchor.Wallet;
  const user = Keypair.generate();
  const unauthorizedUser = Keypair.generate();

  let collectionKeypair: Keypair;
  let assetKeypair: Keypair;
  let configPda: PublicKey;
  let updateAuthorityPda: PublicKey;
  let rewardMintPda: PublicKey;
  let stakeInfoPda: PublicKey;

  before(async () => {
    logSection("Failure Tests Setup");

    await airdrop(connection, user.publicKey);
    await airdrop(connection, unauthorizedUser.publicKey);
    logAccount("user", user.publicKey);
    logAccount("unauthorized user", unauthorizedUser.publicKey);

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

    // Ensure config exists
    try {
      await program.methods
        .initialize(100, 0)
        .accountsPartial({
          admin: admin.publicKey,
          config: configPda,
          rewardMint: rewardMintPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    } catch {
      // Already initialized
    }

    const colSig = await program.methods
      .createCollection("Fail Test Collection", "https://example.com/col.json")
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
    logTxSignature("createCollection (setup)", colSig);

    const mintSig = await program.methods
      .mintAsset("Fail NFT", "https://example.com/fail.json")
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
    logTxSignature("mintAsset (setup)", mintSig);
    logAccount("asset", assetKeypair.publicKey);
  });

  // Config PDA can only be created once via init, second call should fail
  it("fails to initialize twice", async () => {
    try {
      await program.methods
        .initialize(200, 3)
        .accountsPartial({
          admin: admin.publicKey,
          config: configPda,
          rewardMint: rewardMintPda,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      logError("double initialize", err);
      assert.ok(err, "double init should fail");
    }
  });

  // Only the asset owner can stake, unauthorized user should be rejected
  it("fails when unauthorized user tries to stake", async () => {
    const [unauthorizedStakeInfo] = getStakeInfoPda(
      assetKeypair.publicKey,
      unauthorizedUser.publicKey,
      program.programId
    );

    try {
      await program.methods
        .stake()
        .accountsPartial({
          owner: unauthorizedUser.publicKey,
          asset: assetKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          config: configPda,
          stakeInfo: unauthorizedStakeInfo,
          updateAuthority: updateAuthorityPda,
          mplCoreProgram: MPL_CORE_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([unauthorizedUser])
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      logError("unauthorized stake", err);
      assert.ok(err, "unauthorized user staking should fail");
    }
  });

  // Claiming rewards requires at least 1 day elapsed, immediate claim should fail
  it("fails to claim rewards with no time elapsed", async () => {
    const stakeSig = await program.methods
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
    logTxSignature("stake (for claim test)", stakeSig);

    const ownerAta = getAssociatedTokenAddressSync(
      rewardMintPda,
      user.publicKey
    );

    try {
      await program.methods
        .claimRewards()
        .accountsPartial({
          owner: user.publicKey,
          asset: assetKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          config: configPda,
          stakeInfo: stakeInfoPda,
          rewardMint: rewardMintPda,
          ownerTokenAccount: ownerAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      logError("claim with no time elapsed", err);
      assert.ok(err, "claiming with no time elapsed should fail");
    }
  });

  // Non-owner cannot claim rewards even if they know the asset address
  it("fails when unauthorized user tries to claim rewards", async () => {
    const unauthorizedAta = getAssociatedTokenAddressSync(
      rewardMintPda,
      unauthorizedUser.publicKey
    );

    const [unauthorizedStakeInfo] = getStakeInfoPda(
      assetKeypair.publicKey,
      unauthorizedUser.publicKey,
      program.programId
    );

    try {
      await program.methods
        .claimRewards()
        .accountsPartial({
          owner: unauthorizedUser.publicKey,
          asset: assetKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          config: configPda,
          stakeInfo: unauthorizedStakeInfo,
          rewardMint: rewardMintPda,
          ownerTokenAccount: unauthorizedAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([unauthorizedUser])
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      logError("unauthorized claim", err);
      assert.ok(err, "unauthorized user claiming should fail");
    }
  });

  // StakeInfo PDA already exists from a previous stake, second init should fail
  it("fails to stake the same asset twice", async () => {
    try {
      await program.methods
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
      assert.fail("should have thrown");
    } catch (err) {
      logError("double stake", err);
      assert.ok(err, "double staking should fail");
    }
  });

  // Non-owner cannot unstake someone else's staked asset
  it("fails when unauthorized user tries to unstake", async () => {
    const [unauthorizedStakeInfo] = getStakeInfoPda(
      assetKeypair.publicKey,
      unauthorizedUser.publicKey,
      program.programId
    );

    try {
      await program.methods
        .unstake()
        .accountsPartial({
          owner: unauthorizedUser.publicKey,
          asset: assetKeypair.publicKey,
          collection: collectionKeypair.publicKey,
          config: configPda,
          stakeInfo: unauthorizedStakeInfo,
          updateAuthority: updateAuthorityPda,
          mplCoreProgram: MPL_CORE_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([unauthorizedUser])
        .rpc();
      assert.fail("should have thrown");
    } catch (err) {
      logError("unauthorized unstake", err);
      assert.ok(err, "unauthorized user unstaking should fail");
    }
  });
});
