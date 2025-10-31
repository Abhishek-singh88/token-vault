use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked};

declare_id!("EwFjrUDfLEDRA3VHNmGMiG9g4caVraHt47fRiyUP7xuE");

#[program]
pub mod token_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        lock_duration: i64,
    ) -> Result<()> {
        let vault_state = &mut ctx.accounts.vault_state;
        vault_state.owner = ctx.accounts.owner.key();
        vault_state.mint = ctx.accounts.token_mint.key();
        vault_state.vault_token_account = ctx.accounts.vault_token_account.key();
        vault_state.amount_locked = 0;
        vault_state.lock_duration = lock_duration;
        vault_state.locked_at = 0;
        vault_state.is_locked = false;
        vault_state.vault_bump = ctx.bumps.vault_authority;

        msg!("🏦 Vault initialized for mint: {}", ctx.accounts.token_mint.key());
        msg!("🔒 Lock duration: {} seconds", lock_duration);

        Ok(())
    }

    pub fn deposit_tokens(
        ctx: Context<DepositTokens>,
        amount: u64,
    ) -> Result<()> {
        let vault_state = &mut ctx.accounts.vault_state;
        
        //require!(!vault_state.is_locked, VaultError::VaultAlreadyLocked);
        
        // Transfer tokens from user to vault using transfer_checked
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user_authority.to_account_info(),
            mint: ctx.accounts.token_mint.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;

        vault_state.amount_locked += amount;
        vault_state.locked_at = Clock::get()?.unix_timestamp;
        vault_state.is_locked = true;

        msg!("💰 Deposited {} tokens to vault", amount);
        msg!("🔐 Vault is now LOCKED until: {}", 
            vault_state.locked_at + vault_state.lock_duration);

        Ok(())
    }

    pub fn withdraw_tokens(
        ctx: Context<WithdrawTokens>,
        amount: u64,
    ) -> Result<()> {
        let vault_state = &mut ctx.accounts.vault_state;
        
        require!(vault_state.is_locked, VaultError::VaultNotLocked);
        require!(vault_state.amount_locked >= amount, VaultError::InsufficientFunds);
        
        let current_time = Clock::get()?.unix_timestamp;
        let unlock_time = vault_state.locked_at + vault_state.lock_duration;
        
        require!(current_time >= unlock_time, VaultError::TokensStillLocked);

        // Create seeds for PDA signing
        let seeds = &[
            b"vault",
            vault_state.owner.as_ref(),
            vault_state.mint.as_ref(),
            &[vault_state.vault_bump],
        ];
        let signer = &[&seeds[..]];

        // Transfer tokens from vault back to user using transfer_checked
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
            mint: ctx.accounts.token_mint.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        
        transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;

        vault_state.amount_locked -= amount;
        
        if vault_state.amount_locked == 0 {
            vault_state.is_locked = false;
            msg!("🔓 Vault is now UNLOCKED - all tokens withdrawn");
        }

        msg!("💸 Withdrawn {} tokens from vault", amount);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [b"vault-state", owner.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(
        seeds = [b"vault", owner.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    /// CHECK: This is a PDA
    pub vault_authority: AccountInfo<'info>,
    
    #[account(
        init,
        payer = owner,
        token::mint = token_mint,
        token::authority = vault_authority,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositTokens<'info> {
    #[account(
        mut,
        seeds = [b"vault-state", vault_state.owner.as_ref(), vault_state.mint.as_ref()],
        bump
    )]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(
        mut,
        constraint = vault_token_account.key() == vault_state.vault_token_account
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_account.mint == vault_state.mint
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    pub user_authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct WithdrawTokens<'info> {
    #[account(
        mut,
        seeds = [b"vault-state", vault_state.owner.as_ref(), vault_state.mint.as_ref()],
        bump,
        constraint = vault_state.owner == user_authority.key()
    )]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(
        seeds = [b"vault", vault_state.owner.as_ref(), vault_state.mint.as_ref()],
        bump = vault_state.vault_bump
    )]
    /// CHECK: This is a PDA
    pub vault_authority: AccountInfo<'info>,
    
    #[account(
        mut,
        constraint = vault_token_account.key() == vault_state.vault_token_account
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_account.mint == vault_state.mint
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    
    pub token_mint: InterfaceAccount<'info, Mint>,
    
    pub user_authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub vault_token_account: Pubkey,
    pub amount_locked: u64,
    pub lock_duration: i64,
    pub locked_at: i64,
    pub is_locked: bool,
    pub vault_bump: u8,
}

#[error_code]
pub enum VaultError {
    #[msg("Vault is already locked")]
    VaultAlreadyLocked,
    #[msg("Vault is not locked")]
    VaultNotLocked,
    #[msg("Tokens are still locked")]
    TokensStillLocked,
    #[msg("Insufficient funds in vault")]
    InsufficientFunds,
}