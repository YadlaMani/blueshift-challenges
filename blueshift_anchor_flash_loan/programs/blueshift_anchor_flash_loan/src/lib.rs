use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token,TokenAccount,Mint,Transfer,transfer},
    associated_token::AssociatedToken,
};
use anchor_lang::{
   Discriminator,
   solana_program::sysvar::instructions::{
    ID as INSTRUCTIONS_SYSVAR_ID,
    load_instruction_at_checked
   } 
};
declare_id!("22222222222222222222222222222222222222222222");
#[program]
pub mod blueshift_anchor_flash_loan{
    use super::*;
    pub fn borrow(ctx:Context<Loan>,borrow_amount:u64)->Result<()>{
        require!(borrow_amount>0,ProtocolError::InvalidAmount);
        let seeds=&[
            b"protocol".as_ref(),
            &[ctx.bumps.protocol]
        ];
        let signer_seeds=&[&seeds[..]];
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer { from: ctx.accounts.protocol_ata.to_account_info(), to: ctx.accounts.borrower_ata.to_account_info(), authority: ctx.accounts.protocol.to_account_info() },
                signer_seeds
            ),
            borrow_amount
        )?;
        let ixs=ctx.accounts.instructions.to_account_info();
        let instruction_sysvar=ixs.try_borrow_data()?;
        let len=u16::from_le_bytes(instruction_sysvar[0..2].try_into().unwrap());
        if let Ok(repay_ix)=load_instruction_at_checked(len as usize-1,&ixs){
            require_keys_eq!(repay_ix.program_id,ID,ProtocolError::InvalidProgram);
            require!(repay_ix.data[0..8].eq(instruction::Repay::DISCRIMINATOR),ProtocolError::InvalidIX);
            require_keys_eq!(repay_ix.accounts.get(3).ok_or(ProtocolError::InvalidBorrowerAta)?.pubkey,ctx.accounts.borrower_ata.key(),ProtocolError::InvalidBorrowerAta);
            require_keys_eq!(repay_ix.accounts.get(4).ok_or(ProtocolError::InvalidProtocolAta)?.pubkey,ctx.accounts.protocol_ata.key(),ProtocolError::InvalidProtocolAta);

        }else{
            return Err(ProtocolError::MissingRepayInstruction.into());
        }
        Ok(())
     }
     pub fn repay(ctx:Context<Loan>)->Result<()>{
        let ixs=ctx.accounts.instructions.to_account_info();
        let mut amount_borrowed:u64;
        if let Ok(borrow_ix)=load_instruction_at_checked(0,&ixs){
            let mut borrowed_data:[u8;8]=[0u8;8];
            borrowed_data.copy_from_slice(&borrow_ix.data[8..16]);
            amount_borrowed=u64::from_le_bytes(borrowed_data);
        }else{
            return Err(ProtocolError::MissingBorrowInstruction.into());
        }
        let fee:u64=(amount_borrowed as u128).checked_mul(500).unwrap().checked_div(10_000).ok_or(ProtocolError::Overflow)? as u64;
        amount_borrowed=amount_borrowed.checked_add(fee).ok_or(ProtocolError::Overflow)?;
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer{
                    from:ctx.accounts.borrower_ata.to_account_info(),
                    to:ctx.accounts.protocol_ata.to_account_info(),
                    authority:ctx.accounts.borrower.to_account_info()
                },
                
            ),
            amount_borrowed
        )?;
        Ok(()) 
    } 
} 


#[derive(Accounts)]
pub struct Loan<'info> {
  #[account(mut)]
  pub borrower: Signer<'info>,
  #[account(
    seeds = [b"protocol".as_ref()],
    bump,
  )]
  pub protocol: SystemAccount<'info>,
 
  pub mint: Account<'info, Mint>,
  #[account(
    init_if_needed,
    payer = borrower,
    associated_token::mint = mint,
    associated_token::authority = borrower,
  )]
  pub borrower_ata: Account<'info, TokenAccount>,
  #[account(
    mut,
    associated_token::mint = mint,
    associated_token::authority = protocol,
  )]
  pub protocol_ata: Account<'info, TokenAccount>,
 /// CHECK: This account is read-only and is not written to by this instruction.
  #[account(address = INSTRUCTIONS_SYSVAR_ID)]
  pub instructions: UncheckedAccount<'info>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>
}
 #[error_code]
 pub enum ProtocolError {
    #[msg("Invalid Instruction")]
    InvalidIX,
    #[msg("Invalid instruction index")]
    InvalidInstructionIndex,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Not enough funds")]
    NotEnoughFunds,
    #[msg("Program Mismatch")]
    ProgramMismatch,
    #[msg("Invalid program")]
    InvalidProgram,
    #[msg("Invalid borrower ATA")]
    InvalidBorrowerAta,
    #[msg("Invalid protocol ATA")]
    InvalidProtocolAta,
    #[msg("Missing repay instruction")]
    MissingRepayInstruction,
    #[msg("Missing borrow instruction")]
    MissingBorrowInstruction,
    #[msg("Overflow")]
    Overflow,
}