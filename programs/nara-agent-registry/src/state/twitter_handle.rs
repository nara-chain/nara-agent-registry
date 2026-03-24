use anchor_lang::prelude::*;

#[account(zero_copy)]
#[repr(C)]
pub struct TwitterHandle {
    pub agent: Pubkey,
    pub _reserved: [u8; 64],
}
