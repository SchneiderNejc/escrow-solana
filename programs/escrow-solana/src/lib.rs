use anchor_lang::prelude::*;
use anchor_spl::token::{ self, Token, Transfer, TokenAccount, Mint };

declare_id!("9svm47FMtLRAy4TZsAj4fRwhSedKXdHtAMqGQeFUREVr");

#[program]
pub mod simple_escrow {
    use super::*;

    /// Create a new escrow
    pub fn create_escrow(ctx: Context<CreateEscrow>, amount: u64, expiry: i64) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.depositor = ctx.accounts.depositor.key();
        escrow.amount = amount;
        escrow.expiry = Clock::get()?.unix_timestamp + expiry;

        // Convert status to u8
        escrow.status = EscrowStatus::Pending as u8;

        // Assign recipient
        escrow.recipient = ctx.accounts.recipient.key();

        Ok(())
    }

    /// Fund the escrow
    pub fn fund_escrow(ctx: Context<FundEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;

        // Ensure escrow is in pending state
        require!(
            EscrowStatus::try_from(escrow.status)? == EscrowStatus::Pending,
            EscrowError::InvalidStatus
        );

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
        // Clone escrow account info first to avoid overlapping mutable borrow
        let escrow_account_info = ctx.accounts.escrow.to_account_info().clone();

        // Borrow `escrow` mutably after cloning its account info
        let escrow = &mut ctx.accounts.escrow;

        // Ensure escrow is NOT expired
        require!(Clock::get()?.unix_timestamp >= escrow.expiry, EscrowError::EscrowExpired);

        // Ensure caller is the recipient
        require!(ctx.accounts.recipient.key() == escrow.recipient, EscrowError::Unauthorized);

        // Store escrow key in a variable to extend its lifetime
        let escrow_key = escrow.key();
        let escrow_seeds = &[b"escrow", escrow_key.as_ref()];
        let signer_seeds = &[&escrow_seeds[..]];

        // Prepare CPI context for token transfer
        let cpi_accounts = Transfer {
            from: ctx.accounts.escrow_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: escrow_account_info, // Use cloned account info
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds
        );

        // Perform the token transfer
        token::transfer(cpi_ctx, escrow.amount)?;

        // Mark escrow as completed
        escrow.status = EscrowStatus::Completed.as_u8(); // Convert enum to u8

        Ok(())
    }
}

/// Data structure for escrow
#[account]
pub struct Escrow {
    pub depositor: Pubkey, // 32 bytes
    pub recipient: Pubkey, // 32 bytes
    pub amount: u64, // 8 bytes
    pub expiry: i64, // 8 bytes
    pub status: u8, // 1 byte (enum)
    pub bump: u8, // 1 byte
}

impl Escrow {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 1 + 1; // Total: 82 bytes
}

/// Escrow status enum
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
#[repr(u8)]
pub enum EscrowStatus {
    Pending = 0,
    Completed = 1,
    Cancelled = 2,
}

impl EscrowStatus {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for EscrowStatus {
    type Error = ProgramError;

    fn try_from(value: u8) -> core::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(EscrowStatus::Pending),
            1 => Ok(EscrowStatus::Completed),
            2 => Ok(EscrowStatus::Cancelled),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

/// Accounts for creating escrow
#[derive(Accounts)]
pub struct CreateEscrow<'info> {
    #[account(
        init,
        payer = depositor,
        seeds = [b"escrow", depositor.key().as_ref()],
        bump,
        space = 8 + Escrow::LEN // Adjust this space according to the size of your escrow struct
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(
        mut,
        constraint = depositor_token_account.owner == depositor.key(),
        constraint = depositor_token_account.mint == mint.key()
    )]
    pub depositor_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = depositor,
        seeds = [b"escrow-token", escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    #[account(address = token::ID)] // Correct token program address
    pub token_program: Program<'info, Token>,

    // Other accounts
    #[account(mut)]
    pub recipient: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
/// Accounts for funding escrow
#[derive(Accounts)]
pub struct FundEscrow<'info> {
    #[account(mut, has_one = depositor)]
    pub escrow: Account<'info, Escrow>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(mut)]
    pub depositor_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(mut)]
    pub escrow_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, anchor_spl::token::Token>,
}

/// Accounts for withdrawing escrow
#[derive(Accounts)]
pub struct WithdrawEscrow<'info> {
    #[account(mut)]
    pub escrow: Account<'info, Escrow>,

    #[account(mut)]
    pub recipient: Signer<'info>,

    #[account(mut)]
    pub recipient_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(mut)]
    pub escrow_token_account: Account<'info, anchor_spl::token::TokenAccount>,

    #[account(address = anchor_spl::token::ID)]
    pub token_program: Program<'info, anchor_spl::token::Token>,
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
