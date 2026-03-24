use anchor_lang::prelude::*;

#[account(zero_copy)]
#[repr(C)]
pub struct AgentTwitter {
    pub status: u64,
    pub verified_at: i64,
    pub username_len: u64,
    pub tweet_url_len: u64,
    pub username: [u8; 32],
    pub tweet_url: [u8; 256],
    pub _reserved: [u8; 64],
}
