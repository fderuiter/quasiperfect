pub type Uint = bnum::types::U512;
pub type Int = bnum::types::I512;
// No longer using ethnum types.

pub trait UintExt {
    fn from_u64(v: u64) -> Self;
    fn from_u32(v: u32) -> Self;
    fn from_u128(v: u128) -> Self;
    fn from_usize(v: usize) -> Self;
    fn zero() -> Self;
    fn one() -> Self;
    fn as_u128(&self) -> u128;
    fn as_u64(&self) -> u64;
    fn as_u32(&self) -> u32;
    fn as_usize(&self) -> usize;
    fn as_int(&self) -> Int;
    fn as_uint(&self) -> Uint;
}

impl UintExt for Uint {
    fn from_u64(v: u64) -> Self {
        Self::try_from(v).unwrap()
    }
    fn from_u32(v: u32) -> Self {
        Self::try_from(v).unwrap()
    }
    fn from_u128(v: u128) -> Self {
        Self::try_from(v).unwrap()
    }
    fn from_usize(v: usize) -> Self {
        Self::try_from(v).unwrap()
    }
    fn zero() -> Self {
        Self::try_from(0u32).unwrap()
    }
    fn one() -> Self {
        Self::try_from(1u32).unwrap()
    }
    fn as_u128(&self) -> u128 {
        (*self).try_into().unwrap()
    }
    fn as_u64(&self) -> u64 {
        (*self).try_into().unwrap()
    }
    fn as_u32(&self) -> u32 {
        (*self).try_into().unwrap()
    }
    fn as_usize(&self) -> usize {
        (*self).try_into().unwrap()
    }
    fn as_int(&self) -> Int {
        Int::from_le_bytes(self.to_le_bytes())
    }
    fn as_uint(&self) -> Uint {
        *self
    }
}

pub trait IntExt {
    fn from_u64(v: u64) -> Self;
    fn from_u32(v: u32) -> Self;
    fn from_u128(v: u128) -> Self;
    fn zero() -> Self;
    fn one() -> Self;
    fn as_u128(&self) -> u128;
    fn as_u64(&self) -> u64;
    fn as_u32(&self) -> u32;
    fn as_usize(&self) -> usize;
    fn as_int(&self) -> Int;
    fn as_uint(&self) -> Uint;
}

impl IntExt for Int {
    fn from_u64(v: u64) -> Self {
        Self::try_from(v).unwrap()
    }
    fn from_u32(v: u32) -> Self {
        Self::try_from(v).unwrap()
    }
    fn from_u128(v: u128) -> Self {
        Self::try_from(v).unwrap()
    }
    fn zero() -> Self {
        Self::try_from(0u32).unwrap()
    }
    fn one() -> Self {
        Self::try_from(1u32).unwrap()
    }
    fn as_u128(&self) -> u128 {
        (*self).try_into().unwrap()
    }
    fn as_u64(&self) -> u64 {
        (*self).try_into().unwrap()
    }
    fn as_u32(&self) -> u32 {
        (*self).try_into().unwrap()
    }
    fn as_usize(&self) -> usize {
        (*self).try_into().unwrap()
    }
    fn as_uint(&self) -> Uint {
        Uint::from_le_bytes(self.to_le_bytes())
    }
    fn as_int(&self) -> Int {
        *self
    }
}

#[derive(Clone, Debug)]
pub struct PrimePower {
    pub p: u64,
    pub two_e: u32,
    pub val: Uint,
    pub sigma: Uint,
    pub sigma_factors: Vec<Uint>,
    pub needs_rho: Vec<Uint>,
    pub abundance_fp: u128,
}
