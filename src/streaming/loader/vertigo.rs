use super::*;
use crate::arb::VertigoData;

pub struct VertigoLoader;

impl VertigoLoader {
    pub fn get_vertigo(pool_address: &Pubkey) -> Option<VertigoData> {
        if let Some(AccountDataType::VertigoPool(pool_state)) =
            global_data::get_account(pool_address)
        {
            Some(VertigoData {
                pool_address: *pool_address,
                pool_state,
            })
        } else {
            None
        }
    }
}
