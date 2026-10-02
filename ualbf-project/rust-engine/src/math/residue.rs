use crate::obstruction::check_mod_8_factor_native;
use crate::types::Uint;
use crate::types::UintExt;

pub trait IsValidMod8 {
    /// Checks if the integer is congruent to 1 or 3 modulo 8.
    fn is_valid_mod_8(&self) -> bool;
}

impl IsValidMod8 for u64 {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        !check_mod_8_factor_native(*self)
    }
}

impl IsValidMod8 for u32 {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        !check_mod_8_factor_native(*self as u64)
    }
}

impl IsValidMod8 for u128 {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        !check_mod_8_factor_native(*self as u64)
    }
}

impl IsValidMod8 for Uint {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        let low_word = u64::from_le_bytes(self.to_le_bytes()[..8].try_into().unwrap());
        !check_mod_8_factor_native(low_word)
    }
}
