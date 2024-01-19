use anchor_lang::prelude::*;


declare_id!("7cHgqx9oG6SKj2AgXcpX2V8HBUuJku2iT2yNbH7vNppm");

#[program]
pub mod solquad {
  use super::*;

  pub fn initialize_escrow(
    ctx: Context<InitializeEscrow>,
    amount: u64,
  ) -> Result<()> {
    let escrow_account = &mut ctx.accounts.escrow_account;
    escrow_account.escrow_creator = ctx.accounts.escrow_signer.key();
    escrow_account.creator_deposit_amount = amount;
    escrow_account.total_projects = 0;

    Ok(())
  }

  pub fn initialize_pool(ctx: Context<InitializePool>) -> Result<()> {
    let pool_account = &mut ctx.accounts.pool_account;
    pool_account.pool_creator = ctx.accounts.pool_signer.key();
    pool_account.total_projects = 0;
    pool_account.total_votes = 0;

    Ok(())
  }

  pub fn initialize_project(
    ctx: Context<InitializeProject>,
    name: String,
  ) -> Result<()> {
    let project_account = &mut ctx.accounts.project_account;

    project_account.project_owner = ctx.accounts.project_owner.key();
    project_account.project_name = name;
    project_account.votes_count = 0;
    project_account.voter_amount = 0;
    project_account.distributed_amt = 0;
    project_account.is_added_to_pool = false; // test 2

    Ok(())
  }

  pub fn add_project_to_pool(ctx: Context<AddProjectToPool>) -> Result<()> {
    let escrow_account = &mut ctx.accounts.escrow_account;
    let pool_account = &mut ctx.accounts.pool_account;
    let project_account = &mut ctx.accounts.project_account;

    // test 3
    if pool_account
      .projects
      .contains(&project_account.project_owner)
    {
      return Err(SolquadError::AlreadyAssociatedWithPool.into());
    }

    let (expected_address, _bump) = Pubkey::find_program_address(
      &[
        b"project",
        project_account.project_owner.as_ref(),
        pool_account.pool_creator.as_ref(),
      ],
      ctx.program_id,
    );

    // Check if the derived address matches the provided project account's key
    if expected_address != project_account.key() {
      return Err(SolquadError::InvalidProjectAddress.into());
    }

    // test 2
    if project_account.is_added_to_pool {
      return Err(SolquadError::AlreadyAdded.into());
    }

    pool_account.projects.push(project_account.project_owner);
    pool_account.total_projects += 1;

    escrow_account
      .project_reciever_addresses
      .push(project_account.project_owner);

    project_account.is_added_to_pool = true;

    Ok(())
  }

  pub fn vote_for_project(
    ctx: Context<VoteForProject>,
    amount: u64,
  ) -> Result<()> {
    let pool_account = &mut ctx.accounts.pool_account;
    let project_account = &mut ctx.accounts.project_account;

    for i in 0..pool_account.projects.len() {
      if pool_account.projects[i] == project_account.project_owner {
        project_account.votes_count += 1;
        project_account.voter_amount += amount;
      }
    }

    pool_account.total_votes += 1;

    Ok(())
  }

  pub fn distribute_escrow_amount(
    ctx: Context<DistributeEscrowAmount>,
  ) -> Result<()> {
    let escrow_account = &mut ctx.accounts.escrow_account;
    let pool_account = &mut ctx.accounts.pool_account;
    let project_account = &mut ctx.accounts.project_account;

    for i in 0..escrow_account.project_reciever_addresses.len() {
      let project = pool_account.projects[i];
      let votes = match project {
        act if act == project_account.project_owner => {
          project_account.votes_count
        }
        _ => 0,
      };

      // Safe Arithmetic in Escrow Distribution
      let checked_arthimetic = match votes {
        0 => Some(0_u64),
        _ => votes
          .checked_div(pool_account.total_votes)
          .and_then(|ratio| {
            ratio.checked_mul(escrow_account.creator_deposit_amount)
          }),
      };

      match checked_arthimetic {
        Some(amount) => project_account.distributed_amt = amount,
        None => return Err(SolquadError::ArithmeticOverflow.into()),
      }
    }

    Ok(())
  }
}

#[derive(Accounts)]
pub struct InitializeEscrow<'info> {
  #[account(
    init,
    payer = escrow_signer,
    space = 1024,
    seeds = [b"escrow".as_ref(), escrow_signer.key().as_ref()],
    bump,
  )]
  pub escrow_account: Account<'info, Escrow>,
  #[account(mut)]
  pub escrow_signer: Signer<'info>,
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
  #[account(
    init,
    payer = pool_signer,
    space = 1024,
    seeds = [b"pool".as_ref(), pool_signer.key().as_ref()],
    bump,
  )]
  pub pool_account: Account<'info, Pool>,
  #[account(mut)]
  pub pool_signer: Signer<'info>,
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeProject<'info> {
  #[account(
    init,
    payer = project_owner,
    space = 32 + 32 + 8 + 8 + 8 + 8,
    seeds = [b"project".as_ref(), pool_account.key().as_ref(), project_owner.key().as_ref()],
    bump,
  )]
  pub project_account: Account<'info, Project>,
  #[account(mut)]
  pub project_owner: Signer<'info>,
  pub pool_account: Account<'info, Pool>,
  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddProjectToPool<'info> {
  #[account(mut)]
  pub escrow_account: Account<'info, Escrow>,
  #[account(mut)]
  pub pool_account: Account<'info, Pool>,
  pub project_account: Account<'info, Project>,
  pub project_owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct VoteForProject<'info> {
  #[account(mut)]
  pub pool_account: Account<'info, Pool>,
  #[account(mut)]
  pub project_account: Account<'info, Project>,
  #[account(mut)]
  pub voter_sig: Signer<'info>,
}

#[derive(Accounts)]
pub struct DistributeEscrowAmount<'info> {
  #[account(mut)]
  pub escrow_creator: Signer<'info>,
  #[account(mut, has_one = escrow_creator)]
  pub escrow_account: Account<'info, Escrow>,
  #[account(mut)]
  pub pool_account: Account<'info, Pool>,
  #[account(mut)]
  pub project_account: Account<'info, Project>,
}

// Escrow account for quadratic funding
#[account]
pub struct Escrow {
  pub escrow_creator: Pubkey,
  pub creator_deposit_amount: u64,
  pub total_projects: u8,
  pub project_reciever_addresses: Vec<Pubkey>,
}

// Pool for each project
#[account]
pub struct Pool {
  pub pool_creator: Pubkey,
  pub projects: Vec<Pubkey>,
  pub total_projects: u8,
  pub total_votes: u64,
}

// Projects in each pool
#[account]
pub struct Project {
  pub project_owner: Pubkey,
  pub project_name: String,
  pub votes_count: u64,
  pub voter_amount: u64,
  pub distributed_amt: u64,
  pub is_added_to_pool: bool, // test 2
}

// Voters voting for the project
#[account]
pub struct Voter {
  pub voter: Pubkey,
  pub voted_for: Pubkey,
  pub token_amount: u64,
}

#[error_code]
pub enum SolquadError {
  #[msg("Project is already added to a pool")]
  AlreadyAdded,
  #[msg("Already associated with a pool")]
  AlreadyAssociatedWithPool,
  #[msg("Project address is invalid")]
  InvalidProjectAddress,
  #[msg("Arithmetic overflow")]
  ArithmeticOverflow,
}
