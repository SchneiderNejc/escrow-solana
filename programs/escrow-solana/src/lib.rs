use anchor_lang::prelude::*; // Anchor framework for Solana smart contracts
use anchor_spl::token::{ self, Token, Transfer, TokenAccount, Mint }; // SPL token utilities

// Unique program ID for this Solana program
declare_id!("FQsrCdTzAVkqg6eTximoptrxpMERQ5A2uZ6VjcBnGWo9");

/// The escrow_solana program provides functionality to create and manage an escrow mechanism
#[program]
pub mod escrow_solana {
    use super::*;

    /// Creates a new escrow account with specified parameters
    ///
    /// # Arguments
    /// - `ctx`: Context containing accounts and instruction data
    /// - `amount`: The number of tokens to lock in escrow
    /// - `expiry`: The time (in seconds) after which the escrow can be withdrawn
    ///
    /// # Returns
    /// - `Ok(())` if the escrow creation succeeds
    pub fn create_escrow(ctx: Context<CreateEscrow>, amount: u64, expiry: i64) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.depositor = ctx.accounts.depositor.key(); // Set depositor's public key
        escrow.recipient = ctx.accounts.recipient.key(); // Set recipient's public key
        escrow.amount = amount; // Set the amount of tokens for the escrow
        escrow.expiry = Clock::get()?.unix_timestamp + expiry; // Calculate the escrow expiration time
        escrow.status = EscrowStatus::Pending as u8; // Set the initial status to Pending
        Ok(())
    }

    /// Withdraws tokens from an escrow account to the recipient
    ///
    /// # Arguments
    /// - `ctx`: Context containing accounts and instruction data
    ///
    /// # Returns
    /// - `Ok(())` if the withdrawal succeeds
    pub fn withdraw_escrow(ctx: Context<WithdrawEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;

        // Ensure escrow status is valid for withdrawal
        require!(escrow.status == (EscrowStatus::Pending as u8), EscrowError::InvalidStatus);

        // Ensure the escrow has expired before allowing withdrawal
        require!(Clock::get()?.unix_timestamp >= escrow.expiry, EscrowError::EscrowExpired);

        // Transfer tokens from the depositor's account to the recipient's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.depositor_token_account.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.depositor.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, escrow.amount)?;

        escrow.status = EscrowStatus::Completed as u8; // Update the escrow status to Completed
        Ok(())
    }
}

/// The Escrow account stores data about an escrow instance
#[account]
pub struct Escrow {
    pub depositor: Pubkey, // Public key of the depositor
    pub recipient: Pubkey, // Public key of the recipient
    pub amount: u64, // Amount of tokens in escrow
    pub expiry: i64, // Expiry time (timestamp)
    pub status: u8, // Status of the escrow (e.g., Pending, Completed)
}

impl Escrow {
    /// Total space required for the Escrow account
    /// Includes 8 bytes for the account discriminator plus fields
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 1;
}

/// Represents the status of an escrow
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
#[repr(u8)]
pub enum EscrowStatus {
    Pending = 0, // Escrow is awaiting withdrawal
    Completed = 1, // Escrow has been successfully withdrawn
}

/// Accounts required for the `create_escrow` instruction
#[derive(Accounts)]
pub struct CreateEscrow<'info> {
    /// The escrow account being initialized
    #[account(init, payer = depositor, space = Escrow::LEN + 8)]
    pub escrow: Account<'info, Escrow>,

    /// The depositor of the escrow (payer of account rent)
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// The recipient account that will receive tokens
    pub recipient: Account<'info, TokenAccount>,

    /// Token program ID (must match the SPL token program)
    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,

    /// System program (required for account creation)
    pub system_program: Program<'info, System>,

    /// Rent system variable (to check account rent exemption)
    pub rent: Sysvar<'info, Rent>,
}

/// Accounts required for the `withdraw_escrow` instruction
#[derive(Accounts)]
pub struct WithdrawEscrow<'info> {
    /// The escrow account being accessed
    #[account(mut)]
    pub escrow: Account<'info, Escrow>,

    /// The depositor of the escrow (must sign the transaction)
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// Token account of the depositor
    #[account(mut)]
    pub depositor_token_account: Account<'info, TokenAccount>,

    /// Token account of the recipient
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    /// Token program ID (must match the SPL token program)
    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,
}

/// Custom error codes for the escrow program
#[error_code]
pub enum EscrowError {
    #[msg("Invalid escrow status.")] // Error for incorrect status
    InvalidStatus,
    #[msg("Escrow expired.")] // Error if escrow is already expired
    EscrowExpired,
}
