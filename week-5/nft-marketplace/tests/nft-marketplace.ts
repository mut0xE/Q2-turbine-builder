import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftMarketplace } from "../target/types/nft_marketplace";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  fundAccount,
  getListingPda,
  getMarketplacePda,
  getOfferPda,
  getOfferVaultPda,
  getRewardsMintPda,
  getTreasuryPda,
  loadPlayer,
} from "./helper";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  createCollection,
  create as createAsset,
  fetchAssetV1,
  mplCore,
  MPL_CORE_PROGRAM_ID,
} from "@metaplex-foundation/mpl-core";
import {
  generateSigner,
  keypairIdentity,
  publicKey,
  publicKey as umiPublicKey,
} from "@metaplex-foundation/umi";
import {
  fromWeb3JsKeypair,
  toWeb3JsPublicKey,
} from "@metaplex-foundation/umi-web3js-adapters";
import dotenv from "dotenv";
dotenv.config();

// ── Logging Helpers ──────────────────────────────────────────────────

function logTx(label: string, sig: string) {
  console.log(`\n── ${label} ──`);
  console.log(`  tx sig : ${sig}`);
}

function logBalances(
  label: string,
  entries: { name: string; before: number; after: number }[]
) {
  console.log(`  ${label}:`);
  for (const e of entries) {
    const diff = e.after - e.before;
    const sign = diff >= 0 ? "+" : "";
    console.log(
      `    ${e.name.padEnd(12)} | pre: ${(e.before / LAMPORTS_PER_SOL).toFixed(
        6
      )} SOL | post: ${(e.after / LAMPORTS_PER_SOL).toFixed(
        6
      )} SOL | diff: ${sign}${(diff / LAMPORTS_PER_SOL).toFixed(6)} SOL`
    );
  }
}

function logPdas(entries: { name: string; pubkey: PublicKey }[]) {
  console.log("  PDAs:");
  for (const e of entries) {
    console.log(`    ${e.name.padEnd(16)} : ${e.pubkey.toBase58()}`);
  }
}

function logAccounts(entries: { name: string; pubkey: PublicKey }[]) {
  console.log("  Accounts:");
  for (const e of entries) {
    console.log(`    ${e.name.padEnd(16)} : ${e.pubkey.toBase58()}`);
  }
}

describe("nft-marketplace", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.NftMarketplace as Program<NftMarketplace>;
  const connection = provider.connection;

  const MARKET_NAME = "Market" + Date.now().toString().slice(-6);
  const MARKET_FEE = 250; // 2.5%
  const admin = provider.wallet as anchor.Wallet;
  console.log("MAKER_KEY_PATH", process.env.MAKER_KEY_PATH);
  console.log("TAKER_KEY_PATH", process.env.TAKER_KEY_PATH);
  const maker = loadPlayer(process.env.MAKER_KEY_PATH!);
  const taker = loadPlayer(process.env.TAKER_KEY_PATH!);

  let marketplacePda: PublicKey;
  let treasuryPda: PublicKey;
  let rewardsMintPda: PublicKey;
  let paymentMint: PublicKey;
  let collectionPubkey: PublicKey;

  // Separate asset per test flow to avoid conflicts
  let assetForBuySOL: PublicKey;
  let assetForDelist: PublicKey;
  let assetForBuyToken: PublicKey;
  let assetForOffer: PublicKey;
  let assetForAcceptOffer: PublicKey;

  const umi = createUmi(connection.rpcEndpoint, "confirmed").use(mplCore());
  umi.use(
    keypairIdentity(umi.eddsa.createKeypairFromSecretKey(admin.payer.secretKey))
  );

  // UMI — maker is signer for all NFT creations
  const createCoreAsset = async (owner: PublicKey): Promise<PublicKey> => {
    const assetSigner = generateSigner(umi);
    await createAsset(umi, {
      asset: assetSigner,
      owner: umiPublicKey(owner.toBase58()),
      name: "Test Asset",
      uri: "https://example.com/test.json",
    }).sendAndConfirm(umi, {
      send: { skipPreflight: true },
    });
    return new PublicKey(assetSigner.publicKey);
  };

  before(async () => {
    // await fundAccount(connection, admin.payer, maker.publicKey, 3);

    // await fundAccount(connection, admin.payer, taker.publicKey, 2);

    // Derive PDAs
    [marketplacePda] = getMarketplacePda(MARKET_NAME, program.programId);
    [treasuryPda] = getTreasuryPda(admin.publicKey, program.programId);
    [rewardsMintPda] = getRewardsMintPda(marketplacePda, program.programId);

    console.log("\n── Setup ──");
    logAccounts([
      { name: "admin", pubkey: admin.publicKey },
      { name: "maker", pubkey: maker.publicKey },
      { name: "taker", pubkey: taker.publicKey },
    ]);
    logPdas([
      { name: "marketplace", pubkey: marketplacePda },
      { name: "treasury", pubkey: treasuryPda },
      { name: "rewardsMint", pubkey: rewardsMintPda },
    ]);
    console.log(
      `  maker balance : ${(
        (await connection.getBalance(maker.publicKey)) / LAMPORTS_PER_SOL
      ).toFixed(6)} SOL`
    );

    // SPL payment mint for buy_with_token tests
    paymentMint = await createMint(
      connection,
      admin.payer,
      admin.publicKey,
      null,
      6
    );

    // Fund taker with payment tokens
    const takerPaymentAta = await createAssociatedTokenAccount(
      connection,
      admin.payer,
      paymentMint,
      taker.publicKey
    );
    await mintTo(
      connection,
      admin.payer,
      paymentMint,
      takerPaymentAta,
      admin.payer,
      1_000_000_000
    );

    // Create separate assets for each test flow
    assetForBuySOL = await createCoreAsset(maker.publicKey);
    assetForDelist = await createCoreAsset(maker.publicKey);
    assetForBuyToken = await createCoreAsset(maker.publicKey);
    assetForOffer = await createCoreAsset(maker.publicKey);
    assetForAcceptOffer = await createCoreAsset(maker.publicKey);
    console.log("Assets created ✅");
  });

  // ── 1. Initialize ──────────────────────────────────────────────────

  it("initializes the marketplace", async () => {
    const sig = await program.methods
      .initialize(MARKET_NAME, MARKET_FEE)
      .accountsPartial({
        admin: admin.publicKey,
        marketPlace: marketplacePda,
        treasuryPda: treasuryPda,
        rewardsMint: rewardsMintPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    logTx("Initialize Marketplace", sig);
    logPdas([
      { name: "marketplace", pubkey: marketplacePda },
      { name: "treasury", pubkey: treasuryPda },
      { name: "rewardsMint", pubkey: rewardsMintPda },
    ]);

    const mp = await program.account.marketPlace.fetch(marketplacePda);
    assert.equal(mp.fee, MARKET_FEE);
    assert.equal(mp.name, MARKET_NAME);
    assert.equal(mp.admin.toBase58(), admin.publicKey.toBase58());
    console.log(`  fee: ${mp.fee} bps | name: ${mp.name}`);
  });

  // ── 2. List (SOL — no payment_mint) ─────────────────────────────────

  it("lists an NFT for SOL", async () => {
    const price = new anchor.BN(0.001 * LAMPORTS_PER_SOL);
    const [listingPda] = getListingPda(assetForBuySOL, program.programId);

    const makerBalBefore = await connection.getBalance(maker.publicKey);

    const sig = await program.methods
      .list(price)
      .accountsPartial({
        maker: maker.publicKey,
        listing: listingPda,
        asset: assetForBuySOL,
        collection: null,
        marketPlace: marketplacePda,
        paymentMint: null,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    const makerBalAfter = await connection.getBalance(maker.publicKey);

    logTx("List NFT for SOL", sig);
    logPdas([
      { name: "listing", pubkey: listingPda },
      { name: "marketplace", pubkey: marketplacePda },
    ]);
    logAccounts([
      { name: "maker", pubkey: maker.publicKey },
      { name: "asset", pubkey: assetForBuySOL },
    ]);
    logBalances("Balances", [
      { name: "maker", before: makerBalBefore, after: makerBalAfter },
    ]);

    const asset = await fetchAssetV1(umi, publicKey(assetForBuySOL));
    console.log(`  asset owner  : ${asset.owner.toString()}`);
  });
  // ── 3. Buy (SOL) ───────────────────────────────────────────────────

  it("buys NFT with SOL", async () => {
    const [listingPda] = getListingPda(assetForBuySOL, program.programId);
    const takerRewardsAta = getAssociatedTokenAddressSync(
      rewardsMintPda,
      taker.publicKey
    );

    const makerBalBefore = await connection.getBalance(maker.publicKey);
    const takerBalBefore = await connection.getBalance(taker.publicKey);
    const treasuryBalBefore = await connection.getBalance(treasuryPda);

    const sig = await program.methods
      .buy()
      .accountsPartial({
        taker: taker.publicKey,
        maker: maker.publicKey,
        asset: assetForBuySOL,
        collection: null,
        marketPlace: marketplacePda,
        rewardsMint: rewardsMintPda,
        takerRewardsAta: takerRewardsAta,
        listing: listingPda,
        treasury: treasuryPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    const makerBalAfter = await connection.getBalance(maker.publicKey);
    const takerBalAfter = await connection.getBalance(taker.publicKey);
    const treasuryBalAfter = await connection.getBalance(treasuryPda);

    logTx("Buy NFT with SOL", sig);
    logPdas([
      { name: "listing", pubkey: listingPda },
      { name: "marketplace", pubkey: marketplacePda },
      { name: "treasury", pubkey: treasuryPda },
      { name: "rewardsMint", pubkey: rewardsMintPda },
    ]);
    logAccounts([
      { name: "maker", pubkey: maker.publicKey },
      { name: "taker", pubkey: taker.publicKey },
      { name: "asset", pubkey: assetForBuySOL },
      { name: "takerRewardsAta", pubkey: takerRewardsAta },
    ]);
    logBalances("Balances", [
      { name: "maker", before: makerBalBefore, after: makerBalAfter },
      { name: "taker", before: takerBalBefore, after: takerBalAfter },
      { name: "treasury", before: treasuryBalBefore, after: treasuryBalAfter },
    ]);

    // Listing closed
    const listingInfo = await connection.getAccountInfo(listingPda);
    assert.isNull(listingInfo, "listing should be closed after buy");

    await new Promise((resolve) => setTimeout(resolve, 2000));

    // NFT transferred to taker
    const asset = await fetchAssetV1(umi, publicKey(assetForBuySOL));
    assert.equal(asset.owner.toString(), taker.publicKey.toBase58());
    assert.isTrue(makerBalAfter > makerBalBefore, "maker should receive SOL");
    assert.isAbove(treasuryBalAfter, treasuryBalBefore);

    const rewards = await getAccount(connection, takerRewardsAta);
    assert.isTrue(rewards.amount > 0, "taker should receive rewards");
    console.log(`  rewards minted : ${rewards.amount.toString()}`);
    console.log(`  asset owner    : ${asset.owner.toString()}`);
  });

  // ── 4. Delist ───────────────────────────────────────────────────────

  it("delists an NFT", async () => {
    const price = new anchor.BN(2 * LAMPORTS_PER_SOL);
    const [listingPda] = getListingPda(assetForDelist, program.programId);

    // List first
    await program.methods
      .list(price)
      .accountsPartial({
        maker: maker.publicKey,
        listing: listingPda,
        asset: assetForDelist,
        collection: null,
        marketPlace: marketplacePda,
        paymentMint: null,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    const makerBalBefore = await connection.getBalance(maker.publicKey);

    // Delist
    const sig = await program.methods
      .delist()
      .accountsPartial({
        maker: maker.publicKey,
        asset: assetForDelist,
        collection: null,
        marketPlace: marketplacePda,
        listing: listingPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    const makerBalAfter = await connection.getBalance(maker.publicKey);

    logTx("Delist NFT", sig);
    logPdas([
      { name: "listing", pubkey: listingPda },
      { name: "marketplace", pubkey: marketplacePda },
    ]);
    logAccounts([
      { name: "maker", pubkey: maker.publicKey },
      { name: "asset", pubkey: assetForDelist },
    ]);
    logBalances("Balances", [
      { name: "maker", before: makerBalBefore, after: makerBalAfter },
    ]);

    // Listing closed
    const info = await connection.getAccountInfo(listingPda);
    assert.isNull(info, "listing should be closed after delist");

    await new Promise((resolve) => setTimeout(resolve, 2000));

    // NFT back with maker
    const asset = await fetchAssetV1(umi, publicKey(assetForDelist));
    assert.equal(asset.owner.toString(), maker.publicKey.toBase58());
    console.log(`  asset owner  : ${asset.owner.toString()}`);
  });

  // ── 5. Buy with Token ─────────────────────────────────────────────

  it("buy_with_token", async () => {
    const price = new anchor.BN(100_000); // 0.1 tokens (6 decimals)
    const [listingPda] = getListingPda(assetForBuyToken, program.programId);

    // List with payment_mint
    const listSig = await program.methods
      .list(price)
      .accountsPartial({
        maker: maker.publicKey,
        listing: listingPda,
        asset: assetForBuyToken,
        collection: null,
        marketPlace: marketplacePda,
        paymentMint: paymentMint,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();
    logTx("List NFT for Token", listSig);

    // Verify listing has payment_mint set
    const listing = await program.account.listing.fetch(listingPda);
    assert.isNotNull(
      listing.paymentMint,
      "token listing should have payment_mint"
    );
    assert.equal(listing.paymentMint.toBase58(), paymentMint.toBase58());

    // Buy with token
    const takerRewardsAta = getAssociatedTokenAddressSync(
      rewardsMintPda,
      taker.publicKey
    );
    const takerPaymentAta = getAssociatedTokenAddressSync(
      paymentMint,
      taker.publicKey
    );
    const makerPaymentAta = getAssociatedTokenAddressSync(
      paymentMint,
      maker.publicKey
    );
    const treasuryPaymentAta = getAssociatedTokenAddressSync(
      paymentMint,
      treasuryPda,
      true
    );

    const takerBalBefore = await connection.getBalance(taker.publicKey);
    const makerBalBefore = await connection.getBalance(maker.publicKey);
    const takerTokenBefore = (await getAccount(connection, takerPaymentAta))
      .amount;

    const sig = await program.methods
      .buyWithToken()
      .accountsPartial({
        taker: taker.publicKey,
        maker: maker.publicKey,
        asset: assetForBuyToken,
        collection: null,
        marketPlace: marketplacePda,
        paymentMint: paymentMint,
        takerPaymentAta: takerPaymentAta,
        makerPaymentAta: makerPaymentAta,
        treasuryPaymentAta: treasuryPaymentAta,
        treasury: treasuryPda,
        rewardsMint: rewardsMintPda,
        takerRewardsAta: takerRewardsAta,
        listing: listingPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    const takerBalAfter = await connection.getBalance(taker.publicKey);
    const makerBalAfter = await connection.getBalance(maker.publicKey);

    logTx("Buy NFT with Token", sig);
    logPdas([
      { name: "listing", pubkey: listingPda },
      { name: "marketplace", pubkey: marketplacePda },
      { name: "treasury", pubkey: treasuryPda },
      { name: "rewardsMint", pubkey: rewardsMintPda },
    ]);
    logAccounts([
      { name: "maker", pubkey: maker.publicKey },
      { name: "taker", pubkey: taker.publicKey },
      { name: "asset", pubkey: assetForBuyToken },
      { name: "paymentMint", pubkey: paymentMint },
      { name: "takerPaymentAta", pubkey: takerPaymentAta },
      { name: "makerPaymentAta", pubkey: makerPaymentAta },
      { name: "treasuryPayAta", pubkey: treasuryPaymentAta },
    ]);
    logBalances("SOL Balances", [
      { name: "maker", before: makerBalBefore, after: makerBalAfter },
      { name: "taker", before: takerBalBefore, after: takerBalAfter },
    ]);

    // Listing closed
    const listingInfo = await connection.getAccountInfo(listingPda);
    assert.isNull(listingInfo, "listing should be closed after buy_with_token");

    await new Promise((resolve) => setTimeout(resolve, 2000));

    // NFT owned by taker
    const asset = await fetchAssetV1(umi, publicKey(assetForBuyToken));
    assert.equal(asset.owner.toString(), taker.publicKey.toBase58());

    // Maker received tokens
    const makerTokenAccount = await getAccount(connection, makerPaymentAta);
    assert.isAbove(Number(makerTokenAccount.amount), 0);

    const takerTokenAfter = (await getAccount(connection, takerPaymentAta))
      .amount;
    console.log(`  token balances:`);
    console.log(
      `    taker  | pre: ${takerTokenBefore.toString()} | post: ${takerTokenAfter.toString()}`
    );
    console.log(
      `    maker  | received: ${makerTokenAccount.amount.toString()}`
    );
    console.log(`  asset owner    : ${asset.owner.toString()}`);
  });

  // ── 6. Make Offer ─────────────────────────────────────────────────

  it("makes a SOL offer", async () => {
    // List the asset first so offer has a valid listing
    const [listingPda] = getListingPda(assetForOffer, program.programId);

    await program.methods
      .list(new anchor.BN(0.002 * LAMPORTS_PER_SOL))
      .accountsPartial({
        maker: maker.publicKey,
        listing: listingPda,
        asset: assetForOffer,
        collection: null,
        marketPlace: marketplacePda,
        paymentMint: null,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    // Make offer
    const offerAmount = new anchor.BN(0.002 * LAMPORTS_PER_SOL);
    const [offerPda] = getOfferPda(
      assetForOffer,
      taker.publicKey,
      program.programId
    );
    const [vaultPda] = getOfferVaultPda(offerPda, program.programId);

    const takerBalBefore = await connection.getBalance(taker.publicKey);

    const sig = await program.methods
      .makeOffer(offerAmount)
      .accountsPartial({
        taker: taker.publicKey,
        asset: assetForOffer,
        marketPlace: marketplacePda,
        listing: listingPda,
        offer: offerPda,
        offerVault: vaultPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    const takerBalAfter = await connection.getBalance(taker.publicKey);

    logTx("Make SOL Offer", sig);
    logPdas([
      { name: "listing", pubkey: listingPda },
      { name: "offer", pubkey: offerPda },
      { name: "offerVault", pubkey: vaultPda },
      { name: "marketplace", pubkey: marketplacePda },
    ]);
    logAccounts([
      { name: "taker", pubkey: taker.publicKey },
      { name: "asset", pubkey: assetForOffer },
    ]);
    logBalances("Balances", [
      { name: "taker", before: takerBalBefore, after: takerBalAfter },
    ]);

    const offer = await program.account.offer.fetch(offerPda);
    assert.equal(offer.taker.toBase58(), taker.publicKey.toBase58());
    assert.equal(offer.asset.toBase58(), assetForOffer.toBase58());
    assert.equal(offer.amount.toNumber(), offerAmount.toNumber());

    const vaultBalance = await connection.getBalance(vaultPda);
    assert.isAbove(vaultBalance, 0);
    console.log(
      `  vault balance  : ${(vaultBalance / LAMPORTS_PER_SOL).toFixed(6)} SOL`
    );
  });

  // ── 7. Cancel Offer ───────────────────────────────────────────────

  it("cancels the offer and refunds SOL", async () => {
    const [offerPda] = getOfferPda(
      assetForOffer,
      taker.publicKey,
      program.programId
    );
    const [vaultPda] = getOfferVaultPda(offerPda, program.programId);

    const takerBalBefore = await connection.getBalance(taker.publicKey);
    const vaultBalBefore = await connection.getBalance(vaultPda);

    const sig = await program.methods
      .cancelOffer()
      .accountsPartial({
        taker: taker.publicKey,
        asset: assetForOffer,
        offer: offerPda,
        offerVault: vaultPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    const takerBalAfter = await connection.getBalance(taker.publicKey);
    const vaultBalAfter = await connection.getBalance(vaultPda);

    logTx("Cancel Offer", sig);
    logPdas([
      { name: "offer", pubkey: offerPda },
      { name: "offerVault", pubkey: vaultPda },
    ]);
    logAccounts([
      { name: "taker", pubkey: taker.publicKey },
      { name: "asset", pubkey: assetForOffer },
    ]);
    logBalances("Balances", [
      { name: "taker", before: takerBalBefore, after: takerBalAfter },
      { name: "offerVault", before: vaultBalBefore, after: vaultBalAfter },
    ]);

    // Offer closed
    const info = await connection.getAccountInfo(offerPda);
    assert.isNull(info, "offer should be closed after cancel");
    assert.isAbove(takerBalAfter, takerBalBefore);
  });

  // ── 8. Accept Offer ───────────────────────────────────────────────

  it("accepts an offer", async () => {
    const price = new anchor.BN(0.001 * LAMPORTS_PER_SOL);
    const [listingPda] = getListingPda(assetForAcceptOffer, program.programId);
    const [offerPda] = getOfferPda(
      assetForAcceptOffer,
      taker.publicKey,
      program.programId
    );
    const [vaultPda] = getOfferVaultPda(offerPda, program.programId);

    // Maker lists the asset (SOL listing)
    await program.methods
      .list(price)
      .accountsPartial({
        maker: maker.publicKey,
        listing: listingPda,
        asset: assetForAcceptOffer,
        collection: null,
        marketPlace: marketplacePda,
        paymentMint: null,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    // Taker makes an offer
    const offerAmount = new anchor.BN(0.002 * LAMPORTS_PER_SOL);
    await program.methods
      .makeOffer(offerAmount)
      .accountsPartial({
        taker: taker.publicKey,
        asset: assetForAcceptOffer,
        marketPlace: marketplacePda,
        listing: listingPda,
        offer: offerPda,
        offerVault: vaultPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    // Maker accepts
    const makerBalBefore = await connection.getBalance(maker.publicKey);
    const takerBalBefore = await connection.getBalance(taker.publicKey);
    const treasuryBalBefore = await connection.getBalance(treasuryPda);
    const vaultBalBefore = await connection.getBalance(vaultPda);

    const sig = await program.methods
      .acceptOffer()
      .accountsPartial({
        maker: maker.publicKey,
        taker: taker.publicKey,
        asset: assetForAcceptOffer,
        collection: null,
        marketPlace: marketplacePda,
        listing: listingPda,
        offer: offerPda,
        offerVault: vaultPda,
        treasury: treasuryPda,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([maker])
      .rpc();

    const makerBalAfter = await connection.getBalance(maker.publicKey);
    const takerBalAfter = await connection.getBalance(taker.publicKey);
    const treasuryBalAfter = await connection.getBalance(treasuryPda);
    const vaultBalAfter = await connection.getBalance(vaultPda);

    logTx("Accept Offer", sig);
    logPdas([
      { name: "listing", pubkey: listingPda },
      { name: "offer", pubkey: offerPda },
      { name: "offerVault", pubkey: vaultPda },
      { name: "marketplace", pubkey: marketplacePda },
      { name: "treasury", pubkey: treasuryPda },
    ]);
    logAccounts([
      { name: "maker", pubkey: maker.publicKey },
      { name: "taker", pubkey: taker.publicKey },
      { name: "asset", pubkey: assetForAcceptOffer },
    ]);
    logBalances("Balances", [
      { name: "maker", before: makerBalBefore, after: makerBalAfter },
      { name: "taker", before: takerBalBefore, after: takerBalAfter },
      { name: "treasury", before: treasuryBalBefore, after: treasuryBalAfter },
      { name: "offerVault", before: vaultBalBefore, after: vaultBalAfter },
    ]);

    // Both accounts closed
    assert.isNull(await connection.getAccountInfo(listingPda));
    assert.isNull(await connection.getAccountInfo(offerPda));
    assert.isTrue(makerBalAfter > makerBalBefore);

    await new Promise((resolve) => setTimeout(resolve, 2000));

    // NFT owned by taker
    const asset = await fetchAssetV1(umi, publicKey(assetForAcceptOffer));
    assert.equal(asset.owner.toString(), taker.publicKey.toBase58());
    console.log(`  asset owner    : ${asset.owner.toString()}`);
  });
});
