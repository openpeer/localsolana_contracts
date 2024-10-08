use anchor_lang::prelude::*;
use anchor_lang::solana_program::{ program::invoke, system_instruction };
use anchor_spl::token::{ self, Mint, Token, TokenAccount, Transfer };
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::associated_token::{ create, get_associated_token_address };

declare_id!("1w3ekpHrruiEJPYKpQH6rQssTRNKCKiqUjfQeJXTTrX");

#[program]
pub mod local_solana_migrate {
    use super::*;

    pub const DISPUTE_FEE: u64 = 5_000_000;

    pub fn initialize(
        ctx: Context<Initialize>,
        fee_bps: u64,
        dispute_fee: u64,
        fee_discount_nft: Pubkey
    ) -> Result<()> {
        let escrow_state = &mut ctx.accounts.escrow_state;
        require!(!escrow_state.is_initialized, SolanaErrorCode::AlreadyInitialized);
        escrow_state.is_initialized = true;
        escrow_state.seller = *ctx.accounts.seller.key;
        escrow_state.fee_bps = fee_bps;
        escrow_state.arbitrator = *ctx.accounts.arbitrator.key;
        escrow_state.fee_recipient = *ctx.accounts.fee_recipient.key;
        escrow_state.fee_discount_nft = fee_discount_nft;
        escrow_state.dispute_fee = dispute_fee;
        // escrow_state.deployer = *ctx.accounts.deployer.key;
        Ok(())
    }

    pub fn create_escrow_sol(
        ctx: Context<CreateEscrowSOL>,
        order_id: String,
        amount: u64,
        seller_waiting_time: i64,
        automatic_escrow: bool
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);
        require!(
            ctx.accounts.buyer.key() != ctx.accounts.seller.key(),
            SolanaErrorCode::InvalidBuyer
        );
        require!(
            seller_waiting_time >= 15 * 60 && seller_waiting_time <= 24 * 60 * 60,
            SolanaErrorCode::InvalidSellerWaitingTime
        );

        let escrow_account = &mut ctx.accounts.escrow;
        require!(!escrow_account.exists, SolanaErrorCode::OrderAlreadyExists);

        escrow_account.exists = true;
        escrow_account.seller_can_cancel_after = Clock::get()?.unix_timestamp + seller_waiting_time;
        escrow_account.fee = ((amount * ctx.accounts.escrow_state.fee_bps) / 10000) as u64;
        escrow_account.dispute = false;
        escrow_account.partner = ctx.accounts.partner.key();
        escrow_account.open_peer_fee = ((amount * 30) / 10000) as u64;
        escrow_account.automatic_escrow = automatic_escrow;
        escrow_account.amount = amount;
        escrow_account.token = Pubkey::default();
        escrow_account.seller = *ctx.accounts.seller.key;
        escrow_account.buyer = *ctx.accounts.buyer.key;

        let seller_info = ctx.accounts.seller.to_account_info();
        let escrow_info = ctx.accounts.escrow.to_account_info();
        let system_program_info = ctx.accounts.system_program.to_account_info();
        // // amount  = amount+ &ctx.accounts.escrow.open_peer_fee;
        let fee_amount = ctx.accounts.escrow.amount + ctx.accounts.escrow.fee;
        if automatic_escrow {
            //Transfer lamports from seller to escrow account
            invoke(
                &system_instruction::transfer(seller_info.key, escrow_info.key, fee_amount),
                &[seller_info.clone(), escrow_info.clone(), system_program_info.clone()]
            )?;
        }
        emit!(EscrowCreated { order_id });
        Ok(())
    }

    pub fn create_escrow_sol_buyer(
        ctx: Context<CreateEscrowSOLBuyer>,
        order_id: String,
        amount: u64,
        seller_waiting_time: i64,
        automatic_escrow: bool
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);
        require!(
            ctx.accounts.buyer.key() != ctx.accounts.seller.key(),
            SolanaErrorCode::InvalidBuyer
        );
        require!(
            seller_waiting_time >= 15 * 60 && seller_waiting_time <= 24 * 60 * 60,
            SolanaErrorCode::InvalidSellerWaitingTime
        );

        let escrow_account = &mut ctx.accounts.escrow;
        require!(!escrow_account.exists, SolanaErrorCode::OrderAlreadyExists);

        escrow_account.exists = true;
        escrow_account.seller_can_cancel_after = Clock::get()?.unix_timestamp + seller_waiting_time;
        escrow_account.fee = ((amount * ctx.accounts.escrow_state.fee_bps) / 10000) as u64;
        escrow_account.dispute = false;
        escrow_account.partner = ctx.accounts.partner.key();
        escrow_account.open_peer_fee = ((amount * 30) / 10000) as u64;
        escrow_account.automatic_escrow = automatic_escrow;
        escrow_account.amount = amount;
        escrow_account.token = Pubkey::default();
        escrow_account.seller = *ctx.accounts.seller.key;
        escrow_account.buyer = *ctx.accounts.buyer.key;
        // // amount  = amount+ &ctx.accounts.escrow.open_peer_fee;
        let fee_amount = ctx.accounts.escrow.amount + ctx.accounts.escrow.fee;

        if automatic_escrow {
            
            **ctx.accounts.escrow_state.to_account_info().try_borrow_mut_lamports()? -= fee_amount;
            **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? += fee_amount;
        }
        emit!(EscrowCreated { order_id });
        Ok(())
    }

    pub fn create_escrow_token(
        ctx: Context<CreateEscrowToken>,
        order_id: String,
        amount: u64,
        seller_waiting_time: i64,
        automatic_escrow: bool,
        token: Pubkey,
        from_wallet: bool
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);
        require!(
            ctx.accounts.buyer.key() != ctx.accounts.seller.key(),
            SolanaErrorCode::InvalidBuyer
        );
        require!(
            seller_waiting_time >= 15 * 60 && seller_waiting_time <= 24 * 60 * 60,
            SolanaErrorCode::InvalidSellerWaitingTime
        );
        let escrow_account = &mut ctx.accounts.escrow;
        require!(!escrow_account.exists, SolanaErrorCode::OrderAlreadyExists);

        escrow_account.exists = true;
        escrow_account.seller_can_cancel_after = Clock::get()?.unix_timestamp + seller_waiting_time;
        escrow_account.fee = ((amount * ctx.accounts.escrow_state.fee_bps) / 10000) as u64;
        escrow_account.dispute = false;
        escrow_account.partner = ctx.accounts.partner.key();
        escrow_account.open_peer_fee = ((amount * 30) / 10000) as u64;
        escrow_account.automatic_escrow = automatic_escrow;
        escrow_account.amount = amount;
        escrow_account.token = token;
        escrow_account.seller = *ctx.accounts.seller.key;
        escrow_account.buyer = *ctx.accounts.buyer.key;
        msg!("Automatic Escrow: {}", automatic_escrow);
        if automatic_escrow {
            if ctx.accounts.escrow_token_account.to_account_info().try_borrow_data()?.is_empty() {
                msg!(
                    "Escrow's token associated token account is not initialized. Initializing it..."
                );

                // If the token account doesn't exist, create it
                let cpi_accounts = anchor_spl::associated_token::Create {
                    payer: ctx.accounts.fee_payer.to_account_info(),
                    associated_token: ctx.accounts.escrow_token_account.to_account_info(),
                    authority: escrow_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };

                let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                create(cpi_ctx)?;
                msg!("Escrow's USDC associated token account has been successfully initialized.");
            }

            if !from_wallet {
                if
                    ctx.accounts.escrow_state_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info()
                        .try_borrow_data()?
                        .is_empty()
                {
                    msg!(
                        "Escrow state token associated token account is not initialized. Initializing it..."
                    );

                    // If the token account doesn't exist, create it
                    let cpi_accounts = anchor_spl::associated_token::Create {
                        payer: ctx.accounts.fee_payer.to_account_info(),
                        associated_token: ctx.accounts.escrow_state_token_account
                            .as_ref()
                            .ok_or(SolanaErrorCode::AccountError)?
                            .to_account_info(),
                        authority: ctx.accounts.escrow_state.to_account_info(),
                        mint: ctx.accounts.mint_account.to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                        token_program: ctx.accounts.token_program.to_account_info(),
                    };

                    let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                    create(cpi_ctx)?;
                    msg!(
                        "Escrow state token associated token account has been successfully initialized."
                    );
                }
            }
            // Perform the transfer
            let cpi_accounts = if from_wallet {
                Transfer {
                    from: ctx.accounts.seller_token_account.to_account_info(),
                    to: ctx.accounts.escrow_token_account.to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                }
            } else {
                Transfer {
                    from: ctx.accounts.escrow_state_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    to: ctx.accounts.escrow_token_account.to_account_info(),
                    authority: ctx.accounts.escrow_state.to_account_info(),
                }
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            if from_wallet {
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                token::transfer(cpi_ctx, amount + escrow_account.fee)?;
            } else {
                let seller_key = ctx.accounts.seller.key();
                let escrow_state_seeds = &[
                    b"escrow_state",
                    seller_key.as_ref(),
                    &[ctx.bumps.escrow_state],
                ];
                let seeds: &[&[&[u8]]] = &[escrow_state_seeds];
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    cpi_accounts,
                    seeds
                );
                msg!("Sending: {}",amount+escrow_account.fee);
                token::transfer(cpi_ctx, amount + escrow_account.fee)?;
            }
        }

        emit!(EscrowCreated { order_id });
        Ok(())
    }

    pub fn create_escrow_token_buyer(
        ctx: Context<CreateEscrowTokenBuyer>,
        order_id: String,
        amount: u64,
        seller_waiting_time: i64,
        automatic_escrow: bool,
        token: Pubkey
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);
        require!(
            ctx.accounts.buyer.key() != ctx.accounts.seller.key(),
            SolanaErrorCode::InvalidBuyer
        );
        require!(
            seller_waiting_time >= 15 * 60 && seller_waiting_time <= 24 * 60 * 60,
            SolanaErrorCode::InvalidSellerWaitingTime
        );
        let escrow_account = &mut ctx.accounts.escrow;
        require!(!escrow_account.exists, SolanaErrorCode::OrderAlreadyExists);

        escrow_account.exists = true;
        escrow_account.seller_can_cancel_after = Clock::get()?.unix_timestamp + seller_waiting_time;
        escrow_account.fee = ((amount * ctx.accounts.escrow_state.fee_bps) / 10000) as u64;
        escrow_account.dispute = false;
        escrow_account.partner = ctx.accounts.partner.key();
        escrow_account.open_peer_fee = ((amount * 30) / 10000) as u64;
        escrow_account.automatic_escrow = automatic_escrow;
        escrow_account.amount = amount;
        escrow_account.token = token;
        escrow_account.seller = *ctx.accounts.seller.key;
        escrow_account.buyer = *ctx.accounts.buyer.key;
        msg!("Automatic Escrow: {}", automatic_escrow);
            if ctx.accounts.escrow_token_account.to_account_info().try_borrow_data()?.is_empty() {
                msg!(
                    "Escrow's token associated token account is not initialized. Initializing it..."
                );

                // If the token account doesn't exist, create it
                let cpi_accounts = anchor_spl::associated_token::Create {
                    payer: ctx.accounts.fee_payer.to_account_info(),
                    associated_token: ctx.accounts.escrow_token_account.to_account_info(),
                    authority: escrow_account.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };

                let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                create(cpi_ctx)?;
                msg!("Escrow's USDC associated token account has been successfully initialized.");
            }

            // Perform the transfer
            let cpi_accounts = Transfer {
                from: ctx.accounts.escrow_state_token_account
                    .as_ref()
                    .ok_or(SolanaErrorCode::AccountError)?
                    .to_account_info(),
                to: ctx.accounts.escrow_token_account.to_account_info(),
                authority: ctx.accounts.escrow_state.to_account_info(),
            };
            let seller_key = ctx.accounts.seller.key();
                let escrow_state_seeds = &[
                    b"escrow_state",
                    seller_key.as_ref(),
                    &[ctx.bumps.escrow_state],
                ];
                let seeds: &[&[&[u8]]] = &[escrow_state_seeds];
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    cpi_accounts,
                    seeds
                );
                msg!("Sending: {}",amount+escrow_account.fee);
                token::transfer(cpi_ctx, amount + escrow_account.fee)?;

        emit!(EscrowCreated { order_id });
        Ok(())
    }

    pub fn mark_as_paid<'info>(
        ctx: Context<'_, '_, 'info, 'info, MarkAsPaid>,
        order_id: String
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, SolanaErrorCode::EscrowNotFound);
        if escrow.seller_can_cancel_after != 1 {
            escrow.seller_can_cancel_after = 1;
            emit!(SellerCancelDisabled { order_id: order_id });
        }
        Ok(())
    }

    pub fn release_funds(ctx: Context<ReleaseFunds>, order_id: String) -> Result<()> {
        require!(ctx.accounts.escrow.exists, SolanaErrorCode::EscrowNotFound);
        require!(
            ctx.accounts.escrow.seller_can_cancel_after == 1,
            SolanaErrorCode::CannotReleaseFundsYet
        );
        require!(
            &ctx.accounts.escrow_state.fee_recipient == ctx.accounts.fee_recipient.key,
            SolanaErrorCode::InvalidFeeRecepient
        );
        require!(
            ctx.accounts.escrow.buyer == *ctx.accounts.buyer.key,
            SolanaErrorCode::InvalidBuyer
        );

        require!(
            ctx.accounts.escrow.buyer != *ctx.accounts.seller.key,
            SolanaErrorCode::InvalidBuyer
        );

        if ctx.accounts.escrow.token == Pubkey::default() {
            let fee_amount = ctx.accounts.escrow.fee;
            let total_amount = ctx.accounts.escrow.amount + fee_amount;
            let account_size: usize = ctx.accounts.escrow.to_account_info().data_len();

            // Fetch the rent sysvar
            let rent = Rent::get()?;

            // Calculate the rent-exempt reserve for this account
            let rent_exempt_balance = rent.minimum_balance(account_size);

            // Get the actual balance in the account
            let actual_balance = **ctx.accounts.escrow.to_account_info().lamports.borrow();

            // Calculate the total balance (including the rent-exempt amount)
            let total_balance = actual_balance + rent_exempt_balance;
            msg!("Fee Amount:{}", fee_amount);
            msg!("Escrow Lamports: {}", total_balance);
            msg!("Total Amount: {}", total_amount);

            // Check if escrow account has enough lamports
            //require!(total_balance >= total_amount, SolanaErrorCode::InsufficientFunds);

            **ctx.accounts.buyer.to_account_info().try_borrow_mut_lamports()? +=
                ctx.accounts.escrow.amount;
            **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? -=
                ctx.accounts.escrow.amount;
            **ctx.accounts.fee_recipient.to_account_info().try_borrow_mut_lamports()? += fee_amount;
            **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? -= fee_amount;
        } else {
            msg!("Amount is : {}", ctx.accounts.escrow.amount);
            msg!("Fee is : {}", ctx.accounts.escrow.fee);
            // msg!(
            //     "Escrow Token Account Balance: {}",
            //     let escrow_token_account: TokenAccount = TokenAccount::try_from(&ctx.accounts.escrow_token_account.to_account_info())?;
            //     escrow_token_account.amount;
            // );
            let (_escrow_pda, _bump) = Pubkey::find_program_address(
                &[b"escrow", order_id.as_bytes()],
                ctx.program_id
            );

            let cpi_accounts = Transfer {
                from: ctx.accounts.escrow_token_account
                    .as_ref()
                    .ok_or(SolanaErrorCode::AccountError)?
                    .to_account_info(),
                to: ctx.accounts.buyer_token_account
                    .as_ref()
                    .ok_or(SolanaErrorCode::AccountError)?
                    .to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            };

            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

            // Perform the token transfer (with decimals checked)
            token::transfer(
                cpi_ctx.with_signer(&[&[b"escrow", order_id.as_bytes(), &[_bump]]]),
                ctx.accounts.escrow.amount
            )?;
            msg!("Transferred tokens to buyer");
            // Transfer fee tokens from escrow PDA to the fee recipient's associated token account
            let cpi_accounts_fee = Transfer {
                from: ctx.accounts.escrow_token_account
                    .as_ref()
                    .ok_or(SolanaErrorCode::AccountError)?
                    .to_account_info(),
                to: ctx.accounts.fee_recipient_token_account
                    .as_ref()
                    .ok_or(SolanaErrorCode::AccountError)?
                    .to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            };

            let cpi_program2 = ctx.accounts.token_program.to_account_info();
            let cpi_ctx_fee = CpiContext::new(cpi_program2, cpi_accounts_fee);

            // Perform the token transfer for the fee (with decimals checked)
            token::transfer(
                cpi_ctx_fee.with_signer(&[&[b"escrow", order_id.as_bytes(), &[_bump]]]),
                ctx.accounts.escrow.fee
            )?;
        }

        ctx.accounts.escrow.exists = false;
        emit!(Released { order_id: order_id });
        Ok(())
    }

    pub fn buyer_cancel(ctx: Context<CancelEscrow>, order_id: String) -> Result<()> {
        let escrow = &ctx.accounts.escrow;
        require!(escrow.exists, SolanaErrorCode::EscrowNotFound);

        if escrow.token == Pubkey::default() {
            **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? +=
                escrow.amount + escrow.fee;
            **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? -=
                escrow.amount + escrow.fee;
        } else {
            // if let (Some(seller_token_account), Some(escrow_token_account)) = (
            //     &ctx.accounts.seller_token_account,
            //     &ctx.accounts.escrow_token_account,
            // ) {
            let cpi_accounts = Transfer {
                from: escrow.to_account_info(),
                to: ctx.accounts.seller.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, escrow.amount + escrow.fee)?;
            // }
        }

        ctx.accounts.escrow.exists = false;
        emit!(CancelledByBuyer { order_id: order_id });
        Ok(())
    }

    pub fn seller_cancel(ctx: Context<CancelEscrow>, order_id: String) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, SolanaErrorCode::EscrowNotFound);
        require!(
            escrow.seller_can_cancel_after > 1 &&
                escrow.seller_can_cancel_after <= Clock::get()?.unix_timestamp,
            SolanaErrorCode::CannotCancelYet
        );

        if escrow.token == Pubkey::default() {
            **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? +=
                escrow.amount + escrow.fee;
            **escrow.to_account_info().try_borrow_mut_lamports()? -= escrow.amount + escrow.fee;
        } else {
            // if let (Some(seller_token_account), Some(escrow_token_account)) = (
            //     &ctx.accounts.seller_token_account,
            //     &ctx.accounts.escrow_token_account,
            // ) {
            let cpi_accounts = Transfer {
                from: escrow.to_account_info(),
                to: ctx.accounts.seller.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, escrow.amount + escrow.fee)?;
            //}
        }

        escrow.exists = false;
        emit!(CancelledBySeller { order_id: order_id });
        Ok(())
    }

    pub fn open_dispute(ctx: Context<OpenDispute>, order_id: String) -> Result<()> {
        let escrow_state = &ctx.accounts.escrow_state;
        require!(
            ctx.accounts.payer.to_account_info().lamports() >= escrow_state.dispute_fee,
            SolanaErrorCode::InsufficientFundsForDispute
        );

        require!(
            ctx.accounts.payer.key() == ctx.accounts.escrow.buyer.key() ||
                ctx.accounts.payer.key() == ctx.accounts.escrow.seller.key(),
            SolanaErrorCode::InsufficientFundsForDispute
        );

        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, SolanaErrorCode::EscrowNotFound);
        require!(escrow.seller_can_cancel_after == 1, SolanaErrorCode::CannotOpenDisputeYet);

        // Mark the party that opened the dispute
        if ctx.accounts.payer.key() == escrow.buyer.key() {
            escrow.buyer_paid_dispute = true;
        } else if ctx.accounts.payer.key() == escrow.seller.key() {
            escrow.seller_paid_dispute = true;
        }

        // Transfer dispute fee from payer to program
        invoke(
            &system_instruction::transfer(
                ctx.accounts.payer.to_account_info().key,
                escrow.to_account_info().key,
                DISPUTE_FEE
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.escrow_state.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ]
        )?;

        escrow.dispute = true;
        emit!(DisputeOpened {
            order_id: order_id,
            sender: *ctx.accounts.payer.key,
        });
        Ok(())
    }

    pub fn resolve_dispute(
        ctx: Context<ResolveDispute>,
        order_id: String,
        winner: Pubkey
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, SolanaErrorCode::EscrowNotFound);
        require!(escrow.dispute, SolanaErrorCode::DisputeNotOpen);
        require!(
            winner == ctx.accounts.seller.key() || winner == ctx.accounts.buyer.key(),
            SolanaErrorCode::InvalidWinner
        );

        let winner_account = if winner == ctx.accounts.seller.key() {
            &ctx.accounts.seller
        } else {
            &ctx.accounts.buyer
        };
        let arbitrator = ctx.accounts.arbitrator.to_account_info();

        // Transfer 0.005 SOL from program to winner
        invoke(
            &system_instruction::transfer(
                ctx.accounts.escrow_state.to_account_info().key,
                &winner_account.key(),
                DISPUTE_FEE
            ),
            &[
                ctx.accounts.escrow_state.to_account_info(),
                winner_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ]
        )?;

        // Transfer 0.005 SOL from program to arbitrator
        invoke(
            &system_instruction::transfer(
                ctx.accounts.escrow_state.to_account_info().key,
                arbitrator.key,
                DISPUTE_FEE
            ),
            &[
                ctx.accounts.escrow_state.to_account_info(),
                arbitrator,
                ctx.accounts.system_program.to_account_info(),
            ]
        )?;

        if escrow.token == Pubkey::default() {
            if winner == ctx.accounts.buyer.key() {
                **ctx.accounts.buyer.to_account_info().try_borrow_mut_lamports()? += escrow.amount;
            } else {
                **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? += escrow.amount;
            }
            **escrow.to_account_info().try_borrow_mut_lamports()? -= escrow.amount;
        } else {
            let to_account_info = if winner == ctx.accounts.buyer.key() {
                ctx.accounts.buyer.to_account_info()
            } else {
                ctx.accounts.seller.to_account_info()
            };
            let cpi_accounts = Transfer {
                from: escrow.to_account_info(),
                to: to_account_info,
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, escrow.amount)?;
        }

        escrow.exists = false;
        emit!(DisputeResolved { order_id, winner });
        Ok(())
    }

    pub fn deposit_to_escrow_state(
        ctx: Context<DepositToEscrowState>,
        amount: u64,
        token: Pubkey
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);
        let escrow_state = &ctx.accounts.escrow_state;
        let fee = escrow_state.fee_bps;
        let total_amount = amount + fee;
        msg!("Fee Amount:{}", fee);
        msg!("Total Amount: {}", total_amount);
        if token == Pubkey::default() {
            require!(
                ctx.accounts.seller.to_account_info().lamports() >= total_amount,
                SolanaErrorCode::InsufficientFunds
            );
            **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? -= total_amount;
            **ctx.accounts.escrow_state.to_account_info().try_borrow_mut_lamports()? +=
                total_amount;
            Ok(())
        } else {
            msg!("Starting Token transfer");

            // Get the associated token account for the wallet (sender)
            let wallet_token_account = get_associated_token_address(
                &ctx.accounts.seller.key(),
                &ctx.accounts.mint_account.key()
            );

            // Derive the associated token account for the escrow PDA (receiver)
            let escrow_token_account = get_associated_token_address(
                &ctx.accounts.escrow_state.key(),
                &ctx.accounts.mint_account.key()
            );

            // Log the derived token accounts for debugging
            msg!("Wallet USDC Token Account: {}", wallet_token_account);
            msg!("Escrow USDC Token Account: {}", escrow_token_account);

            // Check if the escrow_state's associated token account already exists
            if
                ctx.accounts.escrow_state_token_account
                    .to_account_info()
                    .try_borrow_data()?
                    .is_empty()
            {
                msg!(
                    "Escrow's USDC associated token account is not initialized. Initializing it..."
                );

                // If the token account doesn't exist, create it
                let cpi_accounts = anchor_spl::associated_token::Create {
                    payer: ctx.accounts.fee_payer.to_account_info(),
                    associated_token: ctx.accounts.escrow_state_token_account.to_account_info(),
                    authority: ctx.accounts.escrow_state.to_account_info(),
                    mint: ctx.accounts.mint_account.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };

                let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                create(cpi_ctx)?;
                msg!("Escrow's USDC associated token account has been successfully initialized.");
            }

            let cpi_accounts = Transfer {
                from: ctx.accounts.wallet_token_account.to_account_info(),
                to: ctx.accounts.escrow_state_token_account.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            msg!("After CPI accounts: {}", ctx.accounts.token_program.key());
            let cpi_program = ctx.accounts.token_program.to_account_info();
            msg!("After cpi program");
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            msg!("After cpi ctx");
            token::transfer(cpi_ctx, total_amount)?;

            Ok(())
        }
    }

    pub fn deposit_to_escrow(
        ctx: Context<DepositToEscrow>,
        order_id: String,
        amount: u64,
        token: Pubkey,
        instant_escrow: bool
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);
        let escrow_account = &ctx.accounts.escrow;
        let amount = escrow_account.amount;
        let fee = escrow_account.fee;
        let total_amount = amount + fee;
        msg!("Fee Amount:{}", fee);
        msg!("Total Amount: {}", total_amount);
        if token == Pubkey::default() {
            require!(
                ctx.accounts.escrow_state.to_account_info().lamports() >= total_amount,
                SolanaErrorCode::InsufficientFunds
            );
            if instant_escrow {
                **ctx.accounts.escrow_state.to_account_info().try_borrow_mut_lamports()? -=
                    total_amount;
                **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? += total_amount;
            } else {
                **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? -= total_amount;
                **ctx.accounts.escrow.to_account_info().try_borrow_mut_lamports()? += total_amount;
            }
        } else {
            if
                ctx.accounts.escrow_token_account
                    .as_ref()
                    .ok_or(SolanaErrorCode::AccountError)?
                    .to_account_info()
                    .try_borrow_data()?
                    .is_empty()
            {
                msg!(
                    "Escrow's token associated token account is not initialized. Initializing it..."
                );

                // If the token account doesn't exist, create it
                let cpi_accounts = anchor_spl::associated_token::Create {
                    payer: ctx.accounts.fee_payer.to_account_info(),
                    associated_token: ctx.accounts.escrow_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    authority: escrow_account.to_account_info(),
                    mint: ctx.accounts.mint_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };

                let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

                create(cpi_ctx)?;
                msg!("Escrow's associated token account has been successfully initialized.");
            }
            if instant_escrow {
                let cpi_accounts = Transfer {
                    from: ctx.accounts.escrow_state_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    to: ctx.accounts.escrow_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                token::transfer(cpi_ctx, total_amount)?;
            } else {
                let cpi_accounts = Transfer {
                    from: ctx.accounts.seller_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    to: ctx.accounts.escrow_token_account
                        .as_ref()
                        .ok_or(SolanaErrorCode::AccountError)?
                        .to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                token::transfer(cpi_ctx, total_amount)?;
            }
        }
        msg!("Transferred: {}", order_id);
        Ok(())
    }

    pub fn withdraw_balance(
        ctx: Context<WithdrawBalance>,
        amount: u64,
        token: Pubkey
    ) -> Result<()> {
        require!(amount > 0, SolanaErrorCode::InvalidAmount);

        if token == Pubkey::default() {
            **ctx.accounts.escrow_state.to_account_info().try_borrow_mut_lamports()? -= amount;
            **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? += amount;
        } else {
            // if let (Some(seller_token_account), Some(escrow_token_account)) = (
            //     &ctx.accounts.seller_token_account,
            //     &ctx.accounts.escrow_token_account,
            // ) {
            let cpi_accounts = Transfer {
                from: ctx.accounts.escrow_state.to_account_info(),
                to: ctx.accounts.seller.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;
            //}
        }

        Ok(())
    }
}

#[derive(Accounts)]
//#[instruction(bump: u8)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 177,
        seeds = [b"escrow_state", seller.key().as_ref()],
        bump
    )]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut)]
    /// CHECK: This is safe because the seller is a trusted party
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: This is safe because the arbitrator is a trusted party
    pub arbitrator: AccountInfo<'info>,
    /// CHECK: This is safe because the fee_recipient is a trusted party
    pub fee_recipient: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(order_id: String, bump: u8)]
pub struct CreateEscrowSOL<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 165 + 8,
        seeds = [b"escrow", order_id.as_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: This is safe because the seller is being checked in program
    pub seller: Signer<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: This is safe because the buyer is a trusted party
    pub buyer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is safe because the partner is a trusted party
    pub partner: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(order_id: String, bump: u8)]
pub struct CreateEscrowSOLBuyer<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 165 + 8,
        seeds = [b"escrow", order_id.as_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: This is safe because the seller is being checked in program
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: This is safe because the buyer is a trusted party
    #[account(mut)]
    pub buyer: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is safe because the partner is a trusted party
    pub partner: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(order_id: String, bump: u8)]
pub struct CreateEscrowToken<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 165 + 8,
        seeds = [b"escrow", order_id.as_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: This is safe because the seller is being checked in program
    pub seller: Signer<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: This is safe because the buyer is a trusted party
    pub buyer: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK This is safe because the escrow token account is being verified in program
    pub escrow_token_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK: This is safe because the partner is a trusted party
    pub partner: AccountInfo<'info>,
    pub mint_account: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = seller
    )]
    pub seller_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
    )]
    /// CHECK: This is safe because the escrow_state_token_account is derived from the escrow_state
    pub escrow_state_token_account: Option<UncheckedAccount<'info>>,
    /// Associated Token Program for creating token accounts
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System Program (needed to create token accounts)
    pub system_program: Program<'info, System>,
    /// Rent sysvar (for rent exemption)
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(order_id: String, bump: u8)]
pub struct CreateEscrowTokenBuyer<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 165 + 8,
        seeds = [b"escrow", order_id.as_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    /// CHECK: This is safe because the seller is being checked in program
    pub seller: AccountInfo<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: This is safe because the buyer is a trusted party
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    /// CHECK This is safe because the escrow token account is being verified in program
    pub escrow_token_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    /// CHECK: This is safe because the partner is a trusted party
    pub partner: AccountInfo<'info>,
    pub mint_account: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = seller
    )]
    pub seller_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
    )]
    /// CHECK: This is safe because the escrow_state_token_account is derived from the escrow_state
    pub escrow_state_token_account: Option<UncheckedAccount<'info>>,
    /// Associated Token Program for creating token accounts
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System Program (needed to create token accounts)
    pub system_program: Program<'info, System>,
    /// Rent sysvar (for rent exemption)
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(order_id: String)]
pub struct MarkAsPaid<'info> {
    #[account(mut, seeds = [b"escrow", order_id.as_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: This is safe because the seller is a trusted party
    pub seller: AccountInfo<'info>,
    // System program is required for PDA derivation
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(order_id: String)]
pub struct ReleaseFunds<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut, seeds = [b"escrow", order_id.as_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is safe because the buyer is validated by the program
    #[account(mut)]
    pub buyer: AccountInfo<'info>,
    /// CHECK: This is safe because the Fee Recipient is validated by the program
    #[account(mut)]
    pub fee_recipient: AccountInfo<'info>,
    #[account(mut,)]
    pub fee_recipient_token_account: Option<UncheckedAccount<'info>>,

    pub token_program: Program<'info, Token>,
    /// CHECK: This is safe because the Fee Recipient is validated by the program
    pub mint_account: UncheckedAccount<'info>,
    #[account(
        mut,
    )]
    pub escrow_token_account: Option<UncheckedAccount<'info>>,
    #[account(
        mut,
    )]
    pub buyer_token_account: Option<UncheckedAccount<'info>>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(order_id: String)]
pub struct CancelEscrow<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut, seeds = [b"escrow", order_id.as_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    pub seller: Signer<'info>,
    // #[account(mut, constraint = escrow_token_account.owner == TOKEN_PROGRAM_ID)]
    // pub escrow_token_account: Option<Account<'info, TokenAccount>>,
    // #[account(mut, constraint = seller_token_account.owner == TOKEN_PROGRAM_ID)]
    // pub seller_token_account: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(order_id: String)]
pub struct OpenDispute<'info> {
    #[account(mut, seeds = [b"escrow_state", payer.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut, seeds = [b"escrow", order_id.as_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    pub payer: Signer<'info>,
    // System program is required for PDA derivation
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(order_id: String)]
pub struct ResolveDispute<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut, seeds = [b"escrow", order_id.as_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    /// CHECK: This is safe because the arbitrator is a trusted party
    pub arbitrator: Signer<'info>,
    /// CHECK: This is safe because the seller is known and validated by the program
    #[account(mut)]
    pub seller: AccountInfo<'info>,
    /// CHECK: This is safe because the buyer is known and validated by the program
    #[account(mut)]
    pub buyer: AccountInfo<'info>,
    // System program is required for PDA derivation
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(order_id: String)]
pub struct DepositToEscrow<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut, seeds = [b"escrow", order_id.as_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub mint_account: Option<Account<'info, Mint>>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = escrow_state
    )]
    pub escrow_state_token_account: Option<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = seller
    )]
    pub seller_token_account: Option<Account<'info, TokenAccount>>,
    /// Associated Token Program for creating token accounts
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System Program (needed to create token accounts)
    pub system_program: Program<'info, System>,
    /// Rent sysvar (for rent exemption)
    pub rent: Sysvar<'info, Rent>,
    #[account(
        mut,
    )]
    /// CHECK: This is safe because the escrow_state_token_account is derived from the escrow_state
    pub escrow_token_account: Option<UncheckedAccount<'info>>,
}

#[derive(Accounts)]
//#[instruction(order_id: String)]
pub struct DepositToEscrowState<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub mint_account: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_account,
        associated_token::authority = seller
    )]
    pub wallet_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
    )]
    /// CHECK: This is safe because the escrow_state_token_account is derived from the escrow_state
    pub escrow_state_token_account: UncheckedAccount<'info>,
    /// Associated Token Program for creating token accounts
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// System Program (needed to create token accounts)
    pub system_program: Program<'info, System>,
    /// Rent sysvar (for rent exemption)
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct WithdrawBalance<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut)]
    pub seller: Signer<'info>,
    // #[account(mut, constraint = escrow_token_account.owner == TOKEN_PROGRAM_ID)]
    // pub escrow_token_account: Option<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    // #[account(mut, constraint = seller_token_account.owner == TOKEN_PROGRAM_ID)]
    // pub seller_token_account: Option<Account<'info, TokenAccount>>,
}

#[account]
pub struct EscrowState {
    pub is_initialized: bool,
    pub seller: Pubkey,
    pub fee_bps: u64,
    pub arbitrator: Pubkey,
    pub fee_recipient: Pubkey,
    pub fee_discount_nft: Pubkey,
    pub dispute_fee: u64,
    pub deployer: Pubkey,
}

#[account]
pub struct Escrow {
    pub exists: bool,
    pub seller_can_cancel_after: i64,
    pub fee: u64,
    pub dispute: bool,
    pub partner: Pubkey,
    pub open_peer_fee: u64,
    pub automatic_escrow: bool,
    pub amount: u64,
    pub token: Pubkey,
    pub seller: Pubkey,
    pub buyer: Pubkey,
    pub seller_paid_dispute: bool,
    pub buyer_paid_dispute: bool,
}

#[error_code]
pub enum SolanaErrorCode {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid seller waiting time")]
    InvalidSellerWaitingTime,
    #[msg("Escrow not found")]
    EscrowNotFound,
    #[msg("Cannot open dispute yet")]
    CannotOpenDisputeYet,
    #[msg("Insufficient funds for dispute")]
    InsufficientFundsForDispute,
    #[msg("Dispute not open")]
    DisputeNotOpen,
    #[msg("Invalid winner")]
    InvalidWinner,
    #[msg("Order already exists")]
    OrderAlreadyExists,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Invalid buyer")]
    InvalidBuyer,
    #[msg("Cannot cancel yet")]
    CannotCancelYet,
    #[msg("Serialization error")]
    SerializationError,
    #[msg("Already initialized")]
    AlreadyInitialized,
    #[msg("Cannot release funds as order is not marked as paid")]
    CannotReleaseFundsYet,
    #[msg("Invalid Fee Recepient")]
    InvalidFeeRecepient,
    #[msg("Invalid Dispute Initiator")]
    InvalidDisputeInitiator,
    #[msg("You missed to pass one important account")]
    AccountError,
}

#[event]
pub struct EscrowCreated {
    pub order_id: String,
}

#[event]
pub struct Released {
    pub order_id: String,
}

#[event]
pub struct SellerCancelDisabled {
    pub order_id: String,
}

#[event]
pub struct CancelledByBuyer {
    pub order_id: String,
}

#[event]
pub struct CancelledBySeller {
    pub order_id: String,
}

#[event]
pub struct DisputeOpened {
    pub order_id: String,
    pub sender: Pubkey,
}

#[event]
pub struct DisputeResolved {
    pub order_id: String,
    pub winner: Pubkey,
}
