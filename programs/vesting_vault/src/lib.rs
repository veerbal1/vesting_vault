use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};

declare_id!("GvzD2zDi4AvLjRA5893csKVsHYezfv6D2B3SpAenSxoi");

#[error_code]
pub enum ErrorCode {
    #[msg("Vault of out tokens")]
    VaultOutOfToken,

    #[msg("Cliff period is Active, You can start claiming after it ends")]
    CliffPeriodNotOver,

    #[msg("All tokens claimed, no vested tokens available")]
    AllTokensClaimed,

    #[msg("Total tokens must be greater than zero")]
    TokensTooLow,

    #[msg("End time must be in the future")]
    EndTimeMustBeInFuture,

    #[msg("Cliff period till must be between started_at and end_at")]
    InvalidCliffPeriod,
}

#[program]
pub mod vesting_vault {

    use anchor_spl::token;

    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault_state = &mut ctx.accounts.vault_state;
        vault_state.vault_token_account = ctx.accounts.vault_token_account.key();
        vault_state.vault_bump = ctx.bumps.vault_token_account;
        vault_state.mint = ctx.accounts.mint.key();
        vault_state.bump = ctx.bumps.vault_state;
        Ok(())
    }

    pub fn initialize_vesting(
        ctx: Context<InitializeVesting>,
        beneficiary: Pubkey,
        token_tokens: u64,
        end_at: i64,
        cliff_period_till: i64,
    ) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        let vesting_account = &mut ctx.accounts.vesting_account;

        require!(token_tokens > 0, ErrorCode::TokensTooLow);
        require!(end_at > current_time, ErrorCode::EndTimeMustBeInFuture);
        require!(cliff_period_till < end_at, ErrorCode::InvalidCliffPeriod);

        vesting_account.beneficiary = beneficiary;
        vesting_account.total_tokens = token_tokens;
        vesting_account.started_at = current_time;
        vesting_account.end_at = end_at;
        vesting_account.claimed_tokens = 0;
        vesting_account.cliff_period_till = cliff_period_till;

        vesting_account.bump = ctx.bumps.vesting_account;

        // CPI
        let vault_token_account = &ctx.accounts.vault_token_account;

        let cpi_accounts = Transfer {
            from: ctx.accounts.admin_token_account.to_account_info(),
            to: vault_token_account.to_account_info(),
            authority: ctx.accounts.admin.to_account_info(),
        };

        let token_program = ctx.accounts.token_program.to_account_info();

        let cpi_context = CpiContext::new(token_program, cpi_accounts);

        token::transfer(cpi_context, token_tokens)?;
        Ok(())
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        let vesting_account = &mut ctx.accounts.vesting_account;
        let current_time = Clock::get()?.unix_timestamp;

        require!(
            vesting_account.cliff_period_till < current_time,
            ErrorCode::CliffPeriodNotOver
        );

        let elapsed_time = (current_time - vesting_account.started_at) as u64;
        let total_time = (vesting_account.end_at - vesting_account.started_at) as u64;

        let total_tokens = vesting_account.total_tokens;
        let claimed_tokens = vesting_account.claimed_tokens;

        let vested_tokens =
            (((total_tokens as u128) * (elapsed_time as u128)) / (total_time as u128)) as u64;
        let vested_tokens = std::cmp::min(vested_tokens, total_tokens);
        let claimable_tokens = vested_tokens - claimed_tokens;

        require!(claimable_tokens > 0, ErrorCode::AllTokensClaimed);

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.beneficiary_token_account.to_account_info(),
            authority: ctx.accounts.vault_token_account.to_account_info(),
        };

        let token_program = ctx.accounts.token_program.to_account_info();

        let vault_state = &ctx.accounts.vault_state;
        let seeds = &[b"vault".as_ref(), &[vault_state.vault_bump]];
        let signer_seeds = &[&seeds[..]];
        let cpi_context = CpiContext::new_with_signer(token_program, cpi_accounts, signer_seeds);

        vesting_account.claimed_tokens += claimable_tokens;

        token::transfer(cpi_context, claimable_tokens)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(beneficiary: Pubkey)]
pub struct InitializeVesting<'info> {
    #[account(init, seeds=[b"vesting", beneficiary.key().as_ref()], bump, payer = admin, space = VestingAccount::LEN)]
    pub vesting_account: Account<'info, VestingAccount>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(mut, token::mint = mint, token::authority = admin)]
    pub admin_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        seeds = [b"vault"],
        bump,
        token::mint = mint,
        token::authority = vault_token_account,
        payer = admin
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct VestingAccount {
    pub beneficiary: Pubkey,
    pub total_tokens: u64,
    pub started_at: i64,
    pub end_at: i64,
    pub claimed_tokens: u64,
    pub cliff_period_till: i64,
    pub bump: u8,
}

impl VestingAccount {
    pub const LEN: usize = 8 + Self::INIT_SPACE;
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub vault_token_account: Pubkey,
    pub vault_bump: u8,
    pub mint: Pubkey,
    pub bump: u8,
}

impl VaultState {
    pub const LEN: usize = 8 + Self::INIT_SPACE;
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(init, seeds = [b"vault_state"], bump, payer = admin, space = VaultState::LEN)]
    pub vault_state: Account<'info, VaultState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(init_if_needed, seeds = [b"vault"], bump, token::mint = mint, token::authority = vault_token_account, payer = admin)]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(seeds = [b"vault_state"], bump = vault_state.bump)]
    pub vault_state: Account<'info, VaultState>,

    #[account(mut, seeds=[b"vesting", beneficiary.key().as_ref()], bump = vesting_account.bump)]
    pub vesting_account: Account<'info, VestingAccount>,

    #[account(mut)]
    pub beneficiary: Signer<'info>,

    pub mint: Account<'info, Mint>,

    #[account(mut, seeds = [b"vault"], bump = vault_state.vault_bump, token::mint = mint, token::authority = vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = beneficiary,
        associated_token::mint = mint,
        associated_token::authority = beneficiary
    )]
    pub beneficiary_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
