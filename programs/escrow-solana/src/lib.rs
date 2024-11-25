use anchor_lang::prelude::*;
use anchor_spl::token::{ self, Token, TokenAccount, Transfer };

declare_id!("9svm47FMtLRAy4TZsAj4fRwhSedKXdHtAMqGQeFUREVr");

#[program]
pub mod simple_escrow {
    use super::*;

    /// Create a new escrow
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        amount: u64,
        expiry: i64 // Hardcoded for simplicity in client-side
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.depositor = ctx.accounts.depositor.key();
        escrow.recipient = ctx.accounts.recipient.key();
        escrow.amount = amount;
        escrow.expiry = Clock::get()?.unix_timestamp + expiry; // Set expiry time
        escrow.status = EscrowStatus::Pending;

        Ok(())
    }

    /// Fund the escrow
    pub fn fund_escrow(ctx: Context<FundEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;

        // Ensure escrow is in pending state
        require!(escrow.status == EscrowStatus::Pending, EscrowError::InvalidStatus);

        // Transfer tokens to PDA escrow account
        let cpi_accounts = Transfer {
            from: ctx.accounts.depositor_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, escrow.amount)?;

        Ok(())
    }

    /// Withdraw funds (called by recipient)
    pub fn withdraw_escrow(ctx: Context<WithdrawEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;

        // Ensure escrow is expired
        require!(Clock::get()?.unix_timestamp < escrow.expiry, EscrowError::EscrowExpired);

        // Ensure caller is the recipient
        require!(ctx.accounts.recipient.key() == escrow.recipient, EscrowError::Unauthorized);

        // Transfer tokens to recipient
        let escrow_key = escrow.key(); // Store in a variable to extend its lifetime
        let escrow_seeds = &[b"escrow", escrow_key.as_ref()];
        let signer_seeds = &[&escrow_seeds[..]];

        let escrow_account_info = ctx.accounts.escrow.to_account_info().clone(); // Clone here
        let cpi_accounts = Transfer {
            from: ctx.accounts.escrow_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: escrow_account_info,
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds
        );

        token::transfer(cpi_ctx, escrow.amount)?;

        // Mark escrow as completed
        escrow.status = EscrowStatus::Completed;

        Ok(())
    }
}

/// Data structure for escrow
#[account]
pub struct Escrow {
    pub depositor: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
    pub expiry: i64,
    pub status: EscrowStatus,
}

/// Escrow status enum
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum EscrowStatus {
    Pending,
    Completed,
    Cancelled,
}

/// Accounts for creating escrow
#[derive(Accounts)]
pub struct CreateEscrow<'info> {
    #[account(init, payer = depositor, space = 8 + std::mem::size_of::<Escrow>())]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    /// CHECK: Recipient does not need to be initialized
    pub recipient: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

/// Accounts for funding escrow
#[derive(Accounts)]
pub struct FundEscrow<'info> {
    #[account(mut)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    #[account(mut)]
    pub depositor_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Accounts for withdrawing escrow
#[derive(Accounts)]
pub struct WithdrawEscrow<'info> {
    #[account(mut)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub recipient: Signer<'info>,
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Error codes
#[error_code]
pub enum EscrowError {
    #[msg("Invalid escrow status.")]
    InvalidStatus,
    #[msg("Unauthorized.")]
    Unauthorized,
    #[msg("Escrow expired.")]
    EscrowExpired,
}
