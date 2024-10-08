import { LocalSolanaMigrate } from "../target/types/local_solana_migrate";
import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, web3, BN, Provider } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import {
  PublicKey,
  SystemProgram,
  Keypair,
  Transaction,
  Connection,
} from "@solana/web3.js";
import { expect } from "chai";
import { ShyftSdk, Network } from "@shyft-to/js";
import bs58 from "bs58";
import idl from "../target/idl/local_solana_migrate.json";
import { IDL } from "@coral-xyz/anchor/dist/cjs/native/system";

describe("local_solana_migrate", () => {
  const connection = new anchor.web3.Connection(
    "https://api.devnet.solana.com/",
    { commitment: "max" }
  );
  const provider = AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace
    .LocalSolanaMigrate as Program<LocalSolanaMigrate>;

  let seller: Keypair;
  let buyer: Keypair;
  let feePayer: Keypair;
  let escrowPda: PublicKey;
  let escrowStatePda: PublicKey;
  let bump: number;
  const orderId = "3968";
  const shyft = new ShyftSdk({
    apiKey: "JwpOxgz2GUG8VMpA",
    network: Network.Devnet,
  });

  const address = "5ma3WQEhs1kimMVqDB8Rc9PceTkEUVkm68A6g6cgxWjJ";
  const addressSeller = "5ma3WQEhs1kimMVqDB8Rc9PceTkEUVkm68A6g6cgxWjJ";
  const addressBuyer = "5ma3WQEhs1kimMVqDB8Rc9PceTkEUVkm68A6g6cgxWjJ";
  const feeRecipientandArbitrator = new PublicKey(address);

  // Helper function to send transaction using Shyft SDK
  const sendTransactionWithShyft = async (
    transaction: Transaction,
    signers: Keypair[]
  ) => {
    const connection = new Connection("https://api.devnet.solana.com");
    const recentBlockhash = await connection.getLatestBlockhash();
    console.log("Recent blockhash: " + recentBlockhash.blockhash);
    transaction.recentBlockhash = recentBlockhash.blockhash;
    transaction.feePayer = new PublicKey(
      "2Hu9fgnKUWyxqGwLVLhoUPsG9PJ15YbNxB8boWmCdSqC"
    );
    //shyft.wallet.transaction({})
    transaction.partialSign(...signers);
    // for (let index = 0; index < signers.length; index++) {
    //   const element = signers[index];

    // }

    // Serialize the transaction
    let serializedTransaction = transaction.serialize({
      requireAllSignatures: false, // or true, based on your use case
      verifySignatures: true, // or true, based on your use case
    });

    // Convert serialized transaction to base64 string
    let base64Transaction = Buffer.from(serializedTransaction).toString(
      "base64"
    );
    console.log("Tansaction is " + base64Transaction);

    const signature = await shyft.txnRelayer.sign({
      encodedTransaction: base64Transaction,
      network: Network.Devnet,
    });
    console.log("Signed Tansaction is " + signature);
    try {
      // const signature = await connection.sendEncodedTransaction(
      //   signedTransaction
      // );
      await connection.confirmTransaction(signature, "confirmed");
      console.log(signature);
      return signature;
    } catch (error) {
      console.error("Error sending transaction:", error);
      throw error;
    }
  };

  before(async () => {
    const secretKeySellerString =
      "2YQ9NWi8xCezNrhFuF9hpCbKJDWzRN5zRYH5jwKsFGzV6NaEH6YLFEGcCFjSrSDzw5qJZk2GPMndkPXh26x3nV1W";
    const secretKeySeller = Uint8Array.from(bs58.decode(secretKeySellerString));

    const secretKeyBuyerString =
      "3scdQ8KKCWnik1sW5ywLxexqAxMuAK3W8hRPzubbuCjoyt7KarLC8QtKHFEfpvxnXUwmLe3ocFyZjUDfELijzbf8";
    const secretKeyBuyer = Uint8Array.from(bs58.decode(secretKeyBuyerString));

    seller = Keypair.fromSecretKey(secretKeySeller);
    buyer = Keypair.fromSecretKey(secretKeyBuyer);

    const [escrowStatePda_, escrowStateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow_state"), seller.publicKey.toBuffer()],
      program.programId
    );
    escrowStatePda = escrowStatePda_;

    const [escrowPda_, escrowBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), Buffer.from(orderId)],
      program.programId
    );
    escrowPda = escrowPda_;
    bump = escrowBump;

    console.log("Escrow PDA:: " + escrowPda);
    console.log("Escrow Bump:: " + escrowBump);
    console.log("Local Wallet " + anchor.Wallet.local().publicKey);

    // await provider.connection.confirmTransaction(
    //   await provider.connection.requestAirdrop(
    //     seller.publicKey,
    //     5 * web3.LAMPORTS_PER_SOL
    //   ),
    //   "confirmed"
    // );
    // await provider.connection.confirmTransaction(
    //   await provider.connection.requestAirdrop(
    //     buyer.publicKey,
    //     10 * web3.LAMPORTS_PER_SOL
    //   ),
    //   "confirmed"
    // );

    // await provider.connection.confirmTransaction(
    //   await provider.connection.requestAirdrop(
    //     feePayer.publicKey,
    //     100 * web3.LAMPORTS_PER_SOL
    //   ),
    //   "confirmed"
    // );
  });

  // it("Initializes escrow state", async () => {
  //   try {
  //       const tx = new Transaction().add(
  //           await program.methods
  //             .initialize(new BN(50), new BN(1000000), feeRecipientandArbitrator)
  //             .accounts({
  //               seller: seller.publicKey,
  //               arbitrator: feeRecipientandArbitrator,
  //               feeRecipient: feeRecipientandArbitrator,
  //             })
  //             //.signers([seller])
  //             .instruction()
  //         );
  //       // await program.methods
  //       // .initialize(new BN(50), new BN(1000000), PublicKey.default)
  //       // .accounts({
  //       //   seller: seller.publicKey,
  //       //   arbitrator: feeRecipientandArbitrator,
  //       //   feeRecipient: feeRecipientandArbitrator,
  //       // })
  //       // .signers([seller])
  //       // .rpc();
  //   await  sendTransactionWithShyft(tx, [seller]);

  //     // await program.rpc.initialize(
  //     //   new BN(50),
  //     //   new BN(1000000),
  //     //   PublicKey.default,
  //     //   {
  //     //     accounts: {
  //     //       seller: seller.publicKey,
  //     //       arbitrator: provider.wallet.publicKey,
  //     //       feeRecipient: provider.wallet.publicKey,
  //     //       deployer: seller.publicKey,
  //     //       escrowState: escrowStatePda,
  //     //       systemProgram: SystemProgram.programId,
  //     //     },
  //     //     signers: [seller],
  //     //   }
  //     // );
  //     console.log("Escrow state initialized");
  //     const escrowState = await program.account.escrowState.fetch(
  //       escrowStatePda
  //     );
  //     console.log("Escrow State::" + escrowState.seller);
  //     expect(escrowState.seller.toBase58() == seller.publicKey.toBase58());
  //   } catch (err) {
  //     console.error("Error initializing escrow state:", err);
  //     throw err;
  //   }
  // });

  // it("Creates an escrow for SOL transfer", async () => {
  //   try {
  //     const tx = await program.methods
  //       .createEscrowSol(
  //         orderId,
  //         new BN(1 * web3.LAMPORTS_PER_SOL),
  //         new BN(20 * 60)
  //       )
  //       .accounts({
  //         buyer: buyer.publicKey,
  //         seller: seller.publicKey,
  //         partner: provider.wallet.publicKey,
  //       })
  //       .signers([seller])
  //       .transaction();
  //     await sendTransactionWithShyft(tx, [seller]);

  //     // const tx = await program.rpc.createEscrowSol(
  //     //   orderId,
  //     //   new BN(1 * web3.LAMPORTS_PER_SOL),
  //     //   new BN(3600),
  //     //   {
  //     //     accounts: {
  //     //       escrowState: escrowStatePda,
  //     //       escrow: escrowPda,
  //     //       seller: seller.publicKey,
  //     //       buyer: buyer.publicKey,
  //     //       systemProgram: SystemProgram.programId,
  //     //       partner: new PublicKey("2Hu9fgnKUWyxqGwLVLhoUPsG9PJ15YbNxB8boWmCdSqC"),
  //     //     },
  //     //     signers: [seller],
  //     //   }
  //     // );

  //     const balance = await provider.connection.getBalance(escrowPda);
  //     console.log("Escrow created with Escrow:", balance);
  //     const escrow = await program.account.escrow.fetch(escrowPda);
  //     expect(escrow.amount.toNumber() == 1 * web3.LAMPORTS_PER_SOL);
  //     // await program.removeEventListener(listener);
  //   } catch (err) {
  //     console.error("Error creating escrow:", err);
  //     throw err;
  //   }
  //  });

  // it("Marks escrow as paid", async () => {
  //   try {
  //     await program.methods.markAsPaid(orderId).
  //       accounts( {
  //         //escrow: escrowPda,
  //         buyer: buyer.publicKey,
  //         seller: seller.publicKey,
  //         //systemProgram: SystemProgram.programId,
  //       }).
  //       signers( [buyer]).transaction();
  //     const escrow = await program.account.escrow.fetch(escrowPda);
  //     expect(escrow.sellerCanCancelAfter.eq(new BN(1)));
  //   } catch (err) {
  //     console.error("Error marking escrow as paid:", err);
  //     if (err.logs) {
  //       console.error("Transaction logs:", err.logs);
  //     }
  //     throw err;
  //   }
  // });
  it("Deposit funds to escrow", async () => {
    try {
      const tx = await program.methods
        .depositToEscrow(new BN(1000), PublicKey.default, orderId)
        .accounts({ seller: seller.publicKey,feePayer:new PublicKey(
          "2Hu9fgnKUWyxqGwLVLhoUPsG9PJ15YbNxB8boWmCdSqC"
        ) }).signers([seller])
        .transaction();
        const finalTx = await sendTransactionWithShyft(tx, [seller]);
      console.log("Funds Escrowedr", tx,finalTx);
    } catch (err) {
      console.error("Error escrowing funds:", err);
      throw err;
    }
  });

  // it("Releases funds to buyer", async () => {
  //   try {
  //     await program.rpc.releaseFunds(orderId, {
  //       accounts: {
  //         escrowState: escrowStatePda,
  //         escrow: escrowPda,
  //         seller: seller.publicKey,
  //         buyer: buyer.publicKey,
  //         feeRecipient: feeRecipientandArbitrator,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //       },
  //       signers: [seller],
  //     });
  //     console.log("Funds released to buyer");
  //   } catch (err) {
  //     console.error("Error releasing funds:", err);
  //     throw err;
  //   }
  // });

  // it("Fetches seller account balance", async () => {
  //   const balance = await provider.connection.getBalance(seller.publicKey);
  //   console.log("Seller balance:", balance);
  //   const balance2 = await provider.connection.getBalance(escrowPda);
  //   console.log("Escrow Now has balance:", balance2);
  //   expect(balance > 0, "Seller should have a positive balance");
  // });

  // it("Fetches buyer account balance", async () => {
  //   const balance = await provider.connection.getBalance(buyer.publicKey);
  //   console.log("Buyer balance:", balance);
  //   expect(balance > 0, "Buyer should have a positive balance");
  // });
});
