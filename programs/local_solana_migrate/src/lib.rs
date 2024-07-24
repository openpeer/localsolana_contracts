use anchor_lang::prelude::*;
use anchor_lang::solana_program::{ program::invoke, system_instruction };
use anchor_spl::token::{ self, Token, Transfer };

declare_id!("CCuEMUp5dNWkfCwLX6zFr96n2hKs9DbmZ9yxjA1pbjyt");

#[program]
pub mod local_solana_migrate {
    use super::*;

    pub  const DISPUTE_FEE: u64 = 5_000_000; 

    pub fn initialize(
        ctx: Context<Initialize>,
        fee_bps: u64,
        dispute_fee: u64,
        fee_discount_nft: Pubkey
    ) -> Result<()> {
        let escrow_state = &mut ctx.accounts.escrow_state;
        require!(!escrow_state.is_initialized, ErrorCode::AlreadyInitialized);
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
        seller_waiting_time: i64
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(ctx.accounts.buyer.key() != ctx.accounts.seller.key(), ErrorCode::InvalidBuyer);
        require!(
            seller_waiting_time >= 15 * 60 && seller_waiting_time <= 24 * 60 * 60,
            ErrorCode::InvalidSellerWaitingTime
        );

        let escrow_account = &mut ctx.accounts.escrow;
        require!(!escrow_account.exists, ErrorCode::OrderAlreadyExists);

        escrow_account.exists = true;
        escrow_account.seller_can_cancel_after = Clock::get()?.unix_timestamp + seller_waiting_time;
        escrow_account.fee = ((amount * ctx.accounts.escrow_state.fee_bps) / 10000) as u64;
        escrow_account.dispute = false;
        escrow_account.partner = ctx.accounts.partner.key();
        escrow_account.open_peer_fee = ((amount * 30) / 10000) as u64;
        escrow_account.automatic_escrow = false;
        escrow_account.amount = amount;
        escrow_account.token = Pubkey::default();
        escrow_account.seller = *ctx.accounts.seller.key;

        let seller_info = ctx.accounts.seller.to_account_info();
        let escrow_info = ctx.accounts.escrow.to_account_info();
        let system_program_info = ctx.accounts.system_program.to_account_info();
        // amount  = amount+ &ctx.accounts.escrow.open_peer_fee;
        let fee_amount = ctx.accounts.escrow.amount + ctx.accounts.escrow.open_peer_fee;

        // Transfer lamports from seller to escrow account
        invoke(
            &system_instruction::transfer(seller_info.key, escrow_info.key, fee_amount),
            &[seller_info.clone(), escrow_info.clone(), system_program_info.clone()]
        )?;
        emit!(EscrowCreated { order_id });
        Ok(())
    }

    pub fn create_escrow_token(
        ctx: Context<CreateEscrowToken>,
        order_id: String,
        amount: u64,
        seller_waiting_time: i64,
        automatic_escrow: bool,
        token: Pubkey
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(ctx.accounts.buyer.key() != ctx.accounts.seller.key(), ErrorCode::InvalidBuyer);
        require!(
            seller_waiting_time >= 15 * 60 && seller_waiting_time <= 24 * 60 * 60,
            ErrorCode::InvalidSellerWaitingTime
        );
        let escrow_account = &mut ctx.accounts.escrow;
        require!(!escrow_account.exists, ErrorCode::OrderAlreadyExists);

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

        if automatic_escrow {
            if
                let (Some(seller_token_account), Some(escrow_token_account)) = (
                    &ctx.accounts.seller_token_account,
                    &ctx.accounts.escrow_token_account,
                )
            {
                let cpi_accounts = Transfer {
                    from: seller_token_account.to_account_info(),
                    to: escrow_token_account.to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                token::transfer(cpi_ctx, amount)?;
            }
        }

        emit!(EscrowCreated { order_id });
        Ok(())
    }

    pub fn mark_as_paid<'info>(
        ctx: Context<'_, '_, 'info, 'info, MarkAsPaid>,
        order_id: String
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, ErrorCode::EscrowNotFound);
        if escrow.seller_can_cancel_after != 1 {
            escrow.seller_can_cancel_after = 1;
            emit!(SellerCancelDisabled { order_id: order_id });
        }
        Ok(())
    }

    pub fn release_funds(ctx: Context<ReleaseFunds>, order_id: String) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, ErrorCode::EscrowNotFound);
        require!(escrow.seller_can_cancel_after == 1, ErrorCode::CannotReleaseFundsYet);
        require!(
            &ctx.accounts.escrow_state.fee_recipient == ctx.accounts.fee_recipient.key,
            ErrorCode::InvalidFeeRecepient
        );

        if escrow.token == Pubkey::default() {
            let fee_amount = escrow.open_peer_fee;
            **ctx.accounts.buyer.to_account_info().try_borrow_mut_lamports()? += escrow.amount;
            **escrow.to_account_info().try_borrow_mut_lamports()? -= escrow.amount;
            **ctx.accounts.fee_recipient.to_account_info().try_borrow_mut_lamports()? += fee_amount;
        } else {
            let cpi_accounts = Transfer {
                from: escrow.to_account_info(),
                to: ctx.accounts.buyer.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, escrow.amount)?;
        }

        escrow.exists = false;
        emit!(Released { order_id: order_id });
        Ok(())
    }

    pub fn buyer_cancel(ctx: Context<CancelEscrow>, order_id: String) -> Result<()> {
        let escrow = &ctx.accounts.escrow;
        require!(escrow.exists, ErrorCode::EscrowNotFound);

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
        require!(escrow.exists, ErrorCode::EscrowNotFound);
        require!(
            escrow.seller_can_cancel_after > 1 &&
                escrow.seller_can_cancel_after <= Clock::get()?.unix_timestamp,
            ErrorCode::CannotCancelYet
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
            ErrorCode::InsufficientFundsForDispute
        );

        require!(
            ctx.accounts.payer.key() == ctx.accounts.escrow.buyer.key() ||
                ctx.accounts.payer.key() == ctx.accounts.escrow.seller.key(),
            ErrorCode::InsufficientFundsForDispute
        );

        let escrow = &mut ctx.accounts.escrow;
        require!(escrow.exists, ErrorCode::EscrowNotFound);
        require!(escrow.seller_can_cancel_after == 1, ErrorCode::CannotOpenDisputeYet);

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
        require!(escrow.exists, ErrorCode::EscrowNotFound);
        require!(escrow.dispute, ErrorCode::DisputeNotOpen);
        require!(
            winner == ctx.accounts.seller.key() || winner == ctx.accounts.buyer.key(),
            ErrorCode::InvalidWinner
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

    pub fn deposit(ctx: Context<Deposit>, amount: u64, token: Pubkey) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        if token == Pubkey::default() {
            require!(
                ctx.accounts.seller.to_account_info().lamports() >= amount,
                ErrorCode::InsufficientFunds
            );
            **ctx.accounts.seller.to_account_info().try_borrow_mut_lamports()? -= amount;
            **ctx.accounts.escrow_state.to_account_info().try_borrow_mut_lamports()? += amount;
        } else {
            let cpi_accounts = Transfer {
                from: ctx.accounts.seller.to_account_info(),
                to: ctx.accounts.escrow_state.to_account_info(),
                authority: ctx.accounts.seller.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;
        }

        Ok(())
    }

    pub fn withdraw_balance(
        ctx: Context<WithdrawBalance>,
        amount: u64,
        token: Pubkey
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

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
        payer = seller,
        space = 8 + 177,
        seeds = [b"escrow_state", seller.key().as_ref()],
        bump
    )]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut)]
    pub seller: Signer<'info>,
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
        payer = seller,
        space = 8 + 165 + 8,
        seeds = [b"escrow", order_id.as_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is safe because the buyer is a trusted party
    pub buyer: AccountInfo<'info>,
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
        payer = seller,
        space = 8 + 165 + 8,
        seeds = [b"escrow", order_id.as_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: This is safe because the buyer is a trusted party
    pub buyer: AccountInfo<'info>,
    #[account(mut)]
    pub escrow_token_account: Option<UncheckedAccount<'info>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub seller_token_account: Option<UncheckedAccount<'info>>,
    /// CHECK: This is safe because the partner is a trusted party
    pub partner: AccountInfo<'info>,
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

    pub token_program: Program<'info, Token>,
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
#[instruction(bump: u8)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"escrow_state", seller.key().as_ref()], bump)]
    pub escrow_state: Account<'info, EscrowState>,
    #[account(mut)]
    pub seller: Signer<'info>,
    // #[account(mut, constraint = escrow_token_account.owner == TOKEN_PROGRAM_ID)]
    // pub escrow_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    // #[account(mut, constraint = seller_token_account.owner == TOKEN_PROGRAM_ID)]
    // pub seller_token_account: Account<'info, TokenAccount>,
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
pub enum ErrorCode {
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
