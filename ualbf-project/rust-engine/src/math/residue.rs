use crate::lean_ffi::check_mod_8;
use crate::types::Uint;
use crate::types::UintExt;

pub trait IsValidMod8 {
    /// Checks if the integer is congruent to 1 or 3 modulo 8.
    /// Uses a native Rust pre-filter followed by the verified Lean FFI call for definitive proof.
    fn is_valid_mod_8(&self) -> bool;
}

impl IsValidMod8 for u64 {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        let r = self & 7;
        if r == 1 || r == 3 {
            check_mod_8(*self)
        } else {
            false
        }
    }
}

impl IsValidMod8 for u32 {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        (*self as u64).is_valid_mod_8()
    }
}

impl IsValidMod8 for u128 {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        let r = (self & 7) as u8;
        if r == 1 || r == 3 {
            if *self <= u64::MAX as u128 {
                check_mod_8(*self as u64)
            } else {
                true
            }
        } else {
            false
        }
    }
}

impl IsValidMod8 for Uint {
    #[inline]
    fn is_valid_mod_8(&self) -> bool {
        let r = self.to_le_bytes()[0] & 7;
        if r == 1 || r == 3 {
            if *self <= Uint::from_u64(u64::MAX) {
                check_mod_8(self.as_u64())
            } else {
                true
            }
        } else {
            false
        }
    }
}
