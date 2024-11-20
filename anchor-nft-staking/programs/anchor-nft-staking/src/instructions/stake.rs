use anchor_lang::prelude::*;
use anchor_spl::{
    metadata::{
        mpl_token_metadata::instructions::{
            FreezeDelegatedAccountCpi, FreezeDelegatedAccountCpiAccounts,
        },
        MasterEditionAccount, Metadata, MetadataAccount,
    },
    token::{approve, Approve, Mint, Token, TokenAccount},
};

use crate::{
    error::StakeError,
    state::{StakeAccount, StakeConfig, UserAccount},
};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint: Account<'info, Mint>, //InterfaceAccount??
    pub collection_mint: Account<'info, Mint>,
    #[account(
    mut,
    associated_token::mint = mint,
    associated_token::authority = user,
  )]
    pub mint_ata: Account<'info, TokenAccount>,
    #[account(
    seeds = [b"metadata", metadata_program.key().as_ref(), mint.key().as_ref()],
    seeds::program = metadata_program.key(),
    bump,
    constraint = metadata.collection.as_ref().unwrap().key.as_ref() == collection_mint.key().as_ref(),
    constraint = metadata.collection.as_ref().unwrap().verified == true,
  )]
    pub metadata: Account<'info, MetadataAccount>,
    #[account(
    seeds = [b"metadata", b"edition",metadata_program.key().as_ref(), mint.key().as_ref()],
    seeds::program = metadata_program.key(),
    bump,
  )]
    pub master_edition: Account<'info, MasterEditionAccount>,
    #[account(
    seeds = [b"config"],
    bump = config_account.bump
  )]
    pub config_account: Account<'info, StakeConfig>,
    #[account(
    init,
    payer = user,
    space = StakeAccount::INIT_SPACE,
    seeds = [b"stake", config_account.key().as_ref(), mint.key().as_ref()],
    bump,
  )]
    pub stake_account: Account<'info, StakeAccount>,
    #[account(
    mut,
    seeds = [b"user", user.key().as_ref()],
    bump = user_account.bump,
  )]
    pub user_account: Account<'info, UserAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub metadata_program: Program<'info, Metadata>,
}

impl<'info> Stake<'info> {
    pub fn stake(&mut self, bumps: &StakeBumps) -> Result<()> {
        require!(
            self.user_account.amount_staked < self.config_account.max_stake,
            StakeError::MaxStakeReached
        );

        self.stake_account.set_inner(StakeAccount {
            owner: self.user.key(),
            mint: self.mint.key(),
            staked_at: Clock::get()?.unix_timestamp,
            bump: bumps.stake_account,
        });

        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Approve {
            to: self.mint_ata.to_account_info(),
            delegate: self.stake_account.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        approve(cpi_ctx, 1)?;

        let metadata_program = &self.metadata_program.to_account_info();

        let freeze_delegate_accounts = FreezeDelegatedAccountCpiAccounts {
            delegate: &self.stake_account.to_account_info(),
            token_account: &self.mint_ata.to_account_info(),
            edition: &self.master_edition.to_account_info(),
            mint: &self.mint.to_account_info(),
            token_program: &self.token_program.to_account_info(),
        };

        let seeds = &[
            b"stake",
            self.config_account.to_account_info().key.as_ref(),
            self.mint.to_account_info().key.as_ref(),
            &[self.stake_account.bump],
        ];

        let signer_seeds = &[&seeds[..]];

        FreezeDelegatedAccountCpi::new(metadata_program, freeze_delegate_accounts)
            .invoke_signed(signer_seeds)?;

        self.user_account.amount_staked += 1;

        Ok(())
    }
}