#![allow(clippy::unnecessary_cast)]
use crate::types::{Uint, UintExt};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

pub static STARTUP_COMPLETE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiError {
    NullPointer,
    InvalidLayout,
    UnverifiedConstant(String),
    PlatformTruncation,
    ArithmeticOverflow,
    DivisionByZero,
}

pub trait TryFromLean: Sized {
    type Error;
    fn try_from_lean(obj: *mut lean_object) -> Result<Self, Self::Error>;
}

pub fn check_platform_limit(val: u64) -> Result<usize, FfiError> {
    if val > usize::MAX as u64 {
        return Err(FfiError::PlatformTruncation);
    }
    Ok(val as usize)
}

pub fn check_verified_bit(val: u64, bit: u8, name: &str) -> Result<(), FfiError> {
    if (val & (1 << bit)) == 0 {
        return Err(FfiError::UnverifiedConstant(name.to_string()));
    }
    Ok(())
}

pub fn handle_verified_bit_err(err: FfiError) {
    if !STARTUP_COMPLETE.load(Ordering::SeqCst) {
        eprintln!("FATAL: {:?}", err);
        std::process::exit(1);
    } else {
        eprintln!("ERROR: {:?}", err);
    }
}

#[repr(C)]
pub struct lean_object {
    _priv: [u8; 0],
}

#[repr(C)]
pub struct lean_external_class {
    _priv: [u8; 0],
}

extern "C" {
    fn lean_initialize_runtime_module();
    fn lean_initialize_thread();

    pub fn lean_register_external_class(
        finalize: extern "C" fn(*mut c_void),
        foreach: extern "C" fn(*mut c_void, usize),
    ) -> *mut lean_external_class;

    pub fn rs_lean_alloc_external(
        cls: *mut lean_external_class,
        data: *mut c_void,
    ) -> *mut lean_object;

    pub fn rs_lean_get_external_data(obj: *mut lean_object) -> *mut c_void;
    pub fn rs_lean_inc(obj: *mut lean_object);
    pub fn rs_lean_dec(obj: *mut lean_object);
    pub fn rs_lean_is_scalar(obj: *mut lean_object) -> bool;
    pub fn rs_lean_ctor_get(obj: *mut lean_object, idx: u32) -> *mut lean_object;
    pub fn initialize_ualbf_UALBF(builtin: u8) -> *mut lean_object;
    pub fn lean_string_cstr(str: *mut lean_object) -> *const std::ffi::c_char;

    pub fn rs_lean_io_result_mk_ok(a: *mut lean_object) -> *mut lean_object;
    pub fn rs_lean_box_uint32(v: u32) -> *mut lean_object;
    pub fn rs_lean_box_bool(v: bool) -> *mut lean_object;
    pub fn rs_lean_box_unit() -> *mut lean_object;
}

include!("ffi_generated.rs");

const _: () = {
    // Check 256-bit integer layout
    if std::mem::size_of::<[u64; 4]>() != 32 {
        panic!("Lean 256-bit integer must be exactly 32 bytes");
    }
    if std::mem::align_of::<[u64; 4]>() != 8 {
        panic!("Lean 256-bit integer must have 8-byte alignment");
    }

    // Check 512-bit integer layout (external object data size and alignment)
    if std::mem::size_of::<[u64; 8]>() != 64 {
        panic!("512-bit representation [u64; 8] must be exactly 64 bytes");
    }
    if std::mem::align_of::<[u64; 8]>() != 8 {
        panic!("512-bit representation [u64; 8] must have 8-byte alignment");
    }

    // Check Rust engine Uint (512-bit) size
    if std::mem::size_of::<crate::types::Uint>() != 64 {
        panic!("Rust engine Uint (512-bit) must be exactly 64 bytes");
    }
};

pub struct LeanObjectWrapper(pub *mut lean_object);

impl LeanObjectWrapper {
    pub fn new(obj: *mut lean_object) -> Self {
        Self(obj)
    }

    pub fn as_ptr(&self) -> *mut lean_object {
        self.0
    }

    pub fn into_raw(self) -> *mut lean_object {
        let ptr = self.0;
        std::mem::forget(self);
        ptr
    }
}

impl Drop for LeanObjectWrapper {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                rs_lean_dec(self.0);
            }
        }
    }
}

pub trait FromLean {
    fn from_lean(obj: *mut lean_object) -> Self;
}

pub trait ToLean {
    fn to_lean(&self) -> LeanObjectWrapper;
}

pub fn words_to_bytes<const N: usize, const B: usize>(w: &[u64; N]) -> [u8; B] {
    let mut bytes = [0u8; B];
    for i in 0..N {
        let start = i * 8;
        if start >= B {
            break;
        }
        let end = std::cmp::min(start + 8, B);
        let chunk = w[i].to_le_bytes();
        bytes[start..end].copy_from_slice(&chunk[..end - start]);
    }
    bytes
}

pub fn bytes_to_words<const B: usize, const N: usize>(bytes: &[u8; B]) -> [u64; N] {
    let mut w = [0u64; N];
    for i in 0..N {
        let start = i * 8;
        if start < B {
            let mut b = [0u8; 8];
            let end = std::cmp::min(start + 8, B);
            b[..end - start].copy_from_slice(&bytes[start..end]);
            w[i] = u64::from_le_bytes(b);
        }
    }
    w
}

impl TryFromLean for Uint {
    type Error = FfiError;
    fn try_from_lean(obj: *mut lean_object) -> Result<Self, Self::Error> {
        if obj.is_null() {
            return Err(FfiError::NullPointer);
        }
        let w = get_u512(obj);
        let bytes = words_to_bytes::<8, 64>(&w);
        Uint::from_le_slice(&bytes).ok_or(FfiError::InvalidLayout)
    }
}
impl FromLean for Uint {
    fn from_lean(obj: *mut lean_object) -> Self {
        let w = get_u512(obj);
        let bytes = words_to_bytes::<8, 64>(&w);
        Uint::from_le_slice(&bytes).unwrap()
    }
}

impl ToLean for Uint {
    fn to_lean(&self) -> LeanObjectWrapper {
        let bytes = self.to_le_bytes();
        let w = bytes_to_words::<64, 8>(&bytes);
        LeanObjectWrapper::new(alloc_u512(w))
    }
}

static mut U512_CLASS: *mut lean_external_class = std::ptr::null_mut();

extern "C" fn u512_finalize(ptr: *mut c_void) {
    unsafe {
        let _ = Box::from_raw(ptr as *mut [u64; 8]);
    }
}

extern "C" fn u512_foreach(_ptr: *mut c_void, _fn: usize) {}

fn init_u512_class() {
    unsafe {
        U512_CLASS = lean_register_external_class(u512_finalize, u512_foreach);
    }
}

pub const ZERO_U512: [u64; 8] = [0; 8];

pub fn alloc_u512(data: [u64; 8]) -> *mut lean_object {
    unsafe {
        let ptr = Box::into_raw(Box::new(data));
        rs_lean_alloc_external(U512_CLASS, ptr as *mut c_void)
    }
}

pub unsafe fn get_u512_ptr(obj: *mut lean_object) -> *const [u64; 8] {
    unsafe { rs_lean_get_external_data(obj) as *const [u64; 8] }
}

pub fn get_u512(obj: *mut lean_object) -> [u64; 8] {
    unsafe {
        let ptr = rs_lean_get_external_data(obj) as *mut [u64; 8];
        *ptr
    }
}

#[no_mangle]
pub extern "C" fn rust_u512_mk(
    w0: u64,
    w1: u64,
    w2: u64,
    w3: u64,
    w4: u64,
    w5: u64,
    w6: u64,
    w7: u64,
) -> *mut lean_object {
    alloc_u512([w0, w1, w2, w3, w4, w5, w6, w7])
}

#[inline(always)]
pub fn is_none(obj: *mut lean_object) -> bool {
    unsafe { rs_lean_is_scalar(obj) }
}

#[inline(always)]
pub fn get_some(obj: *mut lean_object) -> *mut lean_object {
    unsafe { rs_lean_ctor_get(obj, 0) }
}

static LEAN_INIT: Once = Once::new();

pub fn get_logic_hash() -> String {
    unsafe {
        let obj = ualbf_logic_hash;
        let cstr = lean_string_cstr(obj);
        let hash = std::ffi::CStr::from_ptr(cstr)
            .to_string_lossy()
            .into_owned();
        rs_lean_dec(obj);
        hash
    }
}

pub fn run_runtime_parity_check() {
    let baseline_val = get_baseline_min_prime_factors() as u32;
    let prasad_sunitha = get_prasad_sunitha_bound() as u32;
    let div_5_coprime_3 = get_div_5_coprime_3_bound() as u32;

    for i in 0..16u32 {
        let c3 = (i & 1) as u8;
        let c5 = ((i >> 1) & 1) as u8;
        let s3 = ((i >> 2) & 1) as u8;
        let s5 = ((i >> 3) & 1) as u8;

        let ffi_res = unsafe { ualbf_evaluate_baseline_min_ffi(c3, c5, s3, s5) };
        let native_res = if (i & 3) == 0 && (i & 12) == 12 {
            prasad_sunitha
        } else if (i & 1) == 0 && (i & 2) != 0 && (i & 4) != 0 {
            div_5_coprime_3
        } else {
            baseline_val
        };

        if ffi_res != native_res {
            panic!(
                "FATAL: Native baseline_min evaluation mismatch at info_mask {}! FFI: {}, Native: {}",
                i, ffi_res, native_res
            );
        }
    }

    // Active Mathematical Boundary Fuzzing at Startup
    if try_scale_bound_ceil(10, 0) != Err(FfiError::DivisionByZero)
        || try_scale_bound_ceil(10, 1) != Err(FfiError::DivisionByZero)
        || try_scale_bound_ceil(u128::MAX, 2) != Err(FfiError::ArithmeticOverflow)
    {
        std::process::exit(1);
    }

    if Uint::try_from_lean(std::ptr::null_mut()) != Err(FfiError::NullPointer) {
        std::process::exit(1);
    }

    if check_verified_bit(0, 63, "mock_test").is_ok() {
        std::process::exit(1);
    }
    // 0. Verify manifest hash parity
    #[cfg(not(unverified_build))]
    {
        let expected_hash = crate::manifest_constants::MANIFEST_HASH;
        let actual_hash = get_logic_hash();
        if actual_hash != expected_hash {
            panic!(
                "FATAL: Mathematical Bound Synchronization Guardrail Triggered!\n\
                    The running Lean logic binary was compiled with a different bounds manifest.\n\
                    Expected (Engine): {}\n\
                    Actual (Lean)  : {}",
                expected_hash, actual_hash
            );
        }
    }

    // 1. Verify 512-bit integer word ordering
    let mut bytes_n = [0u8; 64];
    for i in 0..8 {
        let mut w = 0x1111111111111111u64 * (i as u64 + 1);
        if i == 7 {
            w &= 0x0FFFFFFFFFFFFFFF;
        } // Prevent overflow when multiplying by 2
        bytes_n[i * 8..(i + 1) * 8].copy_from_slice(&w.to_le_bytes());
    }
    let n = crate::types::Uint::from_le_slice(&bytes_n).unwrap();
    let x = crate::types::Uint::from_u32(1);
    let s = (n * crate::types::Uint::from_u32(2)) + crate::types::Uint::from_u32(1);
    if !verify_identity_lean(&n, &x, false, &s) {
        panic!("FATAL: Initialization failed due to 512-bit integer word-order mismatch.");
    }

    // 2. Validate fixed-point scaling factor
    let expected_k0 = 1u128 << 64;
    let expected_k1 = ((1u128 << 64) as f64 * 3.0 / 2.0).ceil() as u128;
    if get_static_suffix_bound(0) != expected_k0 || get_static_suffix_bound(1) != expected_k1 {
        panic!("FATAL: Initialization failed due to fixed-point scaling factor mismatch.");
    }
}

thread_local! {
    static IS_LEAN_THREAD_INIT: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

pub fn initialize_lean_runtime() {
    LEAN_INIT.call_once(|| unsafe {
        lean_initialize_runtime_module();
        init_u512_class();
        let res = initialize_ualbf_UALBF(1);
        rs_lean_dec(res);
    });
    IS_LEAN_THREAD_INIT.with(|init| {
        if !init.get() {
            unsafe {
                lean_initialize_thread();
            }
            init.set(true);
        }
    });
}

pub fn initialize_lean_worker_thread() {
    unsafe {
        lean_initialize_thread();
    }
}

pub fn check_mod_8(q: u64) -> bool {
    unsafe { ualbf_check_mod_8(q) != 0 }
}

pub fn check_mod_3(p: u64, two_e: u32) -> bool {
    unsafe { ualbf_check_mod_3(p, two_e) != 0 }
}

pub fn check_mod_5(p: u64, two_e: u32) -> bool {
    unsafe { ualbf_check_mod_5(p, two_e) != 0 }
}

pub fn check_mod_9(p: u64, two_e: u32) -> bool {
    unsafe { ualbf_check_mod_9(p, two_e) != 0 }
}

pub fn try_scale_bound_ceil(bound: u128, p: u128) -> Result<u128, FfiError> {
    if p <= 1 {
        return Err(FfiError::DivisionByZero);
    }
    let p_minus_1 = p - 1;
    let num_add = p.checked_sub(2).ok_or(FfiError::ArithmeticOverflow)?;
    let numerator = bound
        .checked_add(num_add)
        .ok_or(FfiError::ArithmeticOverflow)?;
    let division = numerator
        .checked_div(p_minus_1)
        .ok_or(FfiError::DivisionByZero)?;
    let result = bound
        .checked_add(division)
        .ok_or(FfiError::ArithmeticOverflow)?;
    Ok(result)
}

pub fn get_static_suffix_bound(k: u32) -> u128 {
    let w0 = unsafe { ualbf_static_suffix_bound_w0(k) };
    let w1 = unsafe { ualbf_static_suffix_bound_w1(k) };
    ((w1 as u128) << 64) | (w0 as u128)
}

pub fn get_euler_ceiling() -> (Uint, Uint) {
    unsafe {
        use crate::types::UintExt;
        let num = ualbf_euler_ceiling_num;
        let den = ualbf_euler_ceiling_den;
        if let Err(e) = check_verified_bit(num, 63, "euler_ceiling_num") {
            handle_verified_bit_err(e);
        }
        if let Err(e) = check_verified_bit(den, 63, "euler_ceiling_den") {
            handle_verified_bit_err(e);
        }
        (
            Uint::from_u64(num & !(1 << 63)),
            Uint::from_u64(den & !(1 << 63)),
        )
    }
}

pub fn verify_identity_lean(n_l: &Uint, x_l_abs: &Uint, x_l_neg: bool, s_l: &Uint) -> bool {
    let n_l_obj = n_l.to_lean();
    let x_l_obj = x_l_abs.to_lean();
    let s_l_obj = s_l.to_lean();

    unsafe {
        let ok = ualbf_verify_identity(
            n_l_obj.as_ptr(),
            x_l_obj.as_ptr(),
            if x_l_neg { 1 } else { 0 },
            s_l_obj.as_ptr(),
        );
        ok != 0
    }
}

pub fn get_baseline_min_prime_factors() -> usize {
    unsafe {
        let val = ualbf_baseline_min_prime_factors;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_baseline_min_prime_factors") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        match check_platform_limit(unmasked as u64) {
            Ok(v) => v,
            Err(e) => {
                handle_verified_bit_err(e);
                usize::MAX
            }
        }
    }
}

pub fn get_prasad_sunitha_bound() -> usize {
    unsafe {
        let val = ualbf_prasad_sunitha_bound;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_prasad_sunitha_bound") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        match check_platform_limit(unmasked as u64) {
            Ok(v) => v,
            Err(e) => {
                handle_verified_bit_err(e);
                usize::MAX
            }
        }
    }
}

pub fn get_div_5_coprime_3_bound() -> usize {
    unsafe {
        let val = ualbf_div_5_coprime_3_bound;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_div_5_coprime_3_bound") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        match check_platform_limit(unmasked as u64) {
            Ok(v) => v,
            Err(e) => {
                handle_verified_bit_err(e);
                usize::MAX
            }
        }
    }
}

pub fn get_target_abundance_num() -> u64 {
    unsafe {
        let val = ualbf_target_abundance_num;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_target_abundance_num") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        unmasked as u64
    }
}

pub fn get_target_abundance_den() -> u64 {
    unsafe {
        let val = ualbf_target_abundance_den;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_target_abundance_den") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        unmasked as u64
    }
}

pub fn compute_sigma(p: u64, pow: u32) -> Uint {
    compute_sigma_checked(p, pow).unwrap_or_else(|| {
        panic!(
            "compute_sigma overflow: σ({}^{}) does not fit in 256 bits",
            p, pow
        )
    })
}

pub fn compute_sigma_checked(p: u64, pow: u32) -> Option<Uint> {
    unsafe {
        let opt_obj = ualbf_compute_sigma(p, pow as u64);
        if !is_none(opt_obj) {
            let obj = get_some(opt_obj);
            let w = get_u512(obj);
            rs_lean_dec(opt_obj);
            let b = words_to_bytes::<8, 64>(&w);
            Some(Uint::from_le_slice(&b).unwrap())
        } else {
            None
        }
    }
}

pub fn compute_mod_inverse(a_abs: &Uint, a_neg: bool, m: &Uint) -> Option<Uint> {
    unsafe {
        let a_obj = a_abs.to_lean();
        let m_obj = m.to_lean();

        let opt_obj = ualbf_mod_inverse(a_obj.as_ptr(), if a_neg { 1 } else { 0 }, m_obj.as_ptr());

        if !is_none(opt_obj) {
            let obj = get_some(opt_obj);
            let w = get_u512(obj);
            rs_lean_dec(opt_obj);
            let b = words_to_bytes::<8, 64>(&w);
            Some(Uint::from_le_slice(&b).unwrap())
        } else {
            None
        }
    }
}

pub fn cyclotomic_eval(n: u32, p: Uint) -> Option<Uint> {
    if n == 0 {
        return None;
    }
    let mut phi = p.pow(n) - Uint::from_u128(1);
    for d in 1..n {
        if n % d == 0 {
            if let Some(sub_phi) = cyclotomic_eval(d, p) {
                phi /= sub_phi;
            } else {
                return None;
            }
        }
    }
    Some(phi)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() {
        initialize_lean_runtime();
        initialize_lean_worker_thread();
    }
    use super::*;

    #[test]
    fn test_signature_and_alignment_guarantees() {
        setup();
        assert_eq!(
            std::mem::size_of::<[u64; 4]>(),
            32,
            "Lean 256-bit integer must be exactly 32 bytes"
        );
        assert_eq!(
            std::mem::align_of::<[u64; 4]>(),
            8,
            "Lean 256-bit integer must have 8-byte alignment"
        );

        // Native rust engine Uint mapping (bnum U512 is an array of bytes, align 1)
        assert_eq!(
            std::mem::size_of::<Uint>(),
            64,
            "Rust engine Uint (512-bit) must be exactly 64 bytes"
        );
        assert!(
            std::mem::align_of::<Uint>() >= 1,
            "Rust engine Uint alignment is sufficient"
        );
    }

    /// get_baseline_min_prime_factors must return a positive value.
    /// When built without Lean (dummy_ffi.c), the stub returns 7.
    #[test]
    fn test_get_baseline_min_prime_factors_nonzero() {
        setup();
        let value = get_baseline_min_prime_factors();
        assert!(
            value > 0,
            "baseline_min_prime_factors must be positive, got {}",
            value
        );
    }

    /// get_prasad_sunitha_bound must return a positive value.
    /// When built without Lean (dummy_ffi.c), the stub returns 15.
    #[test]
    fn test_get_prasad_sunitha_bound_nonzero() {
        setup();
        let value = get_prasad_sunitha_bound();
        assert!(
            value > 0,
            "prasad_sunitha_bound must be positive, got {}",
            value
        );
    }

    /// The Prasad-Sunitha bound must exceed the baseline minimum prime factors.
    /// This invariant reflects the mathematical requirement that the Prasad-Sunitha
    /// result (coprime-to-15 case) forces a strictly higher prime count floor.
    #[test]
    fn test_prasad_sunitha_bound_exceeds_baseline() {
        setup();
        let baseline = get_baseline_min_prime_factors();
        let ps_bound = get_prasad_sunitha_bound();
        assert!(
            ps_bound > baseline,
            "prasad_sunitha_bound ({}) must be strictly greater than baseline_min_prime_factors ({})",
            ps_bound,
            baseline
        );
    }

    /// Verify the dummy stub value for baseline_min_prime_factors.
    /// When Lean is not present, dummy_ffi.c provides the return value 7.
    #[test]
    fn test_dummy_baseline_min_prime_factors_value() {
        setup();
        let value = get_baseline_min_prime_factors();
        // The dummy stub (dummy_ffi.c) returns 7. The real Lean proof also exports 7.
        assert_eq!(
            value, 7,
            "expected baseline_min_prime_factors == 7, got {}",
            value
        );
    }

    /// Verify the dummy stub value for prasad_sunitha_bound.
    #[test]
    fn test_dummy_prasad_sunitha_bound_value() {
        setup();
        let value = get_prasad_sunitha_bound();
        assert_eq!(value, 15, "expected prasad_sunitha_bound to match 15");
    }

    /// Verify the dummy stub value for div_5_coprime_3_bound.
    #[test]
    fn test_dummy_div_5_coprime_3_bound_value() {
        setup();
        let value = get_div_5_coprime_3_bound();
        assert_eq!(value, 11, "expected div_5_coprime_3_bound to match 11");
    }

    /// Repeated calls to get_baseline_min_prime_factors must return the same value,
    /// since the result comes from a constant C export (or a Lean proof constant).
    #[test]
    fn test_get_baseline_min_prime_factors_idempotent() {
        setup();
        let first = get_baseline_min_prime_factors();
        let second = get_baseline_min_prime_factors();
        assert_eq!(
            first, second,
            "get_baseline_min_prime_factors must be deterministic"
        );
    }

    /// Repeated calls to get_prasad_sunitha_bound must return the same value.
    #[test]
    fn test_get_prasad_sunitha_bound_idempotent() {
        setup();
        let first = get_prasad_sunitha_bound();
        let second = get_prasad_sunitha_bound();
        assert_eq!(
            first, second,
            "get_prasad_sunitha_bound must be deterministic"
        );
    }

    /// Repeated calls to get_div_5_coprime_3_bound must return the same value.
    #[test]
    fn test_get_div_5_coprime_3_bound_idempotent() {
        setup();
        let first = get_div_5_coprime_3_bound();
        let second = get_div_5_coprime_3_bound();
        assert_eq!(
            first, second,
            "get_div_5_coprime_3_bound must be deterministic"
        );
    }

    // -----------------------------------------------------------------------
    // Tests for get_static_suffix_bound (PR change: now computes locally)
    // -----------------------------------------------------------------------

    /// k=0 means no primes accumulated; the bound is just ceil(2^64), which as
    /// a u128 value equals 2^64.
    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_static_suffix_bound_k0() {
        setup();
        let bound = get_static_suffix_bound(0);
        // With no primes, bound = ceil(2^64 as f64) = 2^64
        assert_eq!(bound, 1u128 << 64);
    }

    /// k=1: only the first odd prime (3) is collected.
    /// bound = ceil(2^64 * 3/2) = ceil(27670116110564327424.0) = 27670116110564327424
    #[test]
    #[cfg_attr(unverified_build, ignore)]
    #[cfg_attr(unverified_build, ignore)]
    fn test_static_suffix_bound_k1() {
        setup();
        let bound = get_static_suffix_bound(1);
        let expected = ((1u128 << 64) as f64 * 3.0 / 2.0).ceil() as u128;
        assert_eq!(bound, expected);
        // Must be strictly larger than 2^64
        assert!(bound > 1u128 << 64);
    }

    /// k=2: primes [3, 5].
    /// bound = ceil(2^64 * 3/2 * 5/4)
    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_static_suffix_bound_k2() {
        setup();
        let bound = get_static_suffix_bound(2);
        let expected = ((1u128 << 64) as f64 * 3.0 / 2.0 * 5.0 / 4.0).ceil() as u128;
        assert_eq!(bound, expected);
        assert!(bound > get_static_suffix_bound(1));
    }

    /// k=3: primes [3, 5, 7].
    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_static_suffix_bound_k3() {
        setup();
        let bound = get_static_suffix_bound(3);
        let expected = ((1u128 << 64) as f64 * 3.0 / 2.0 * 5.0 / 4.0 * 7.0 / 6.0).ceil() as u128;
        assert_eq!(bound, expected);
        assert!(bound > get_static_suffix_bound(2));
    }

    /// The function skips 2 (starts at 3) so collected primes are odd primes.
    /// For k=4, primes should be [3, 5, 7, 11].
    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_static_suffix_bound_k4_uses_odd_primes_starting_at_3() {
        setup();
        let bound = get_static_suffix_bound(4);
        // Primes collected: 3, 5, 7, 11 (not 2)
        let expected =
            ((1u128 << 64) as f64 * 3.0 / 2.0 * 5.0 / 4.0 * 7.0 / 6.0 * 11.0 / 10.0).ceil() as u128;
        assert_eq!(bound, expected);
    }

    /// The bound must be monotonically non-decreasing as k grows, because each
    /// additional prime p contributes a factor p/(p-1) >= 1.
    #[test]

    fn test_static_suffix_bound_monotone_increasing() {
        setup();
        let bounds: Vec<u128> = (0..=8).map(get_static_suffix_bound).collect();
        for w in bounds.windows(2) {
            assert!(
                w[1] >= w[0],
                "bound should be non-decreasing: bounds[k+1]={} < bounds[k]={}",
                w[1],
                w[0]
            );
        }
    }

    /// Each factor p/(p-1) is strictly > 1 for any prime p >= 2, so bounds are
    /// strictly increasing.
    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_static_suffix_bound_strictly_increasing_for_k_gt_0() {
        setup();
        for k in 1..=6u32 {
            assert!(
                get_static_suffix_bound(k) > get_static_suffix_bound(k - 1),
                "bound(k={}) should be strictly greater than bound(k={})",
                k,
                k - 1
            );
        }
    }

    /// check_mod_8 edge cases covering mod-8 residues 5 and 7 (the "difficult" cases).
    #[test]
    fn test_check_mod_8_returns_true_for_1_mod_8() {
        setup();
        assert!(check_mod_8(1)); // 1 % 8 = 1
        assert!(check_mod_8(9)); // 9 % 8 = 1
        assert!(check_mod_8(17)); // 17 % 8 = 1
    }

    #[test]
    fn test_check_mod_8_returns_true_for_3_mod_8() {
        setup();
        assert!(check_mod_8(3)); // 3 % 8 = 3
        assert!(check_mod_8(11)); // 11 % 8 = 3
        assert!(check_mod_8(19)); // 19 % 8 = 3
    }

    #[test]
    fn test_check_mod_8_returns_false_for_other_residues() {
        setup();
        assert!(!check_mod_8(5)); // 5 % 8 = 5
        assert!(!check_mod_8(2)); // 2 % 8 = 2
        assert!(!check_mod_8(7)); // 7 % 8 = 7
        assert!(!check_mod_8(8)); // 8 % 8 = 0
        assert!(!check_mod_8(13)); // 13 % 8 = 5
        assert!(!check_mod_8(15)); // 15 % 8 = 7
    }

    #[test]
    fn test_check_mod_8_boundary_zero() {
        setup();
        assert!(!check_mod_8(0)); // 0 % 8 = 0
    }

    #[test]
    #[cfg_attr(unverified_build, ignore)]
    fn test_cyclotomic_eval_arbitrary_degrees() {
        setup();
        use crate::types::UintExt;
        // Test evaluation for degrees > 9 outside the original {3, 5, 7, 9} set.
        let p = Uint::from_u128(2);
        // Phi_10(2) = 2^4 - 2^3 + 2^2 - 2 + 1 = 16 - 8 + 4 - 2 + 1 = 11
        assert_eq!(cyclotomic_eval(10, p).unwrap(), Uint::from_u128(11));
        // Phi_11(2) = 2^10 + 2^9 + ... + 1 = 2047
        assert_eq!(cyclotomic_eval(11, p).unwrap(), Uint::from_u128(2047));
        // Phi_13(2) = 2^12 + ... + 1 = 8191
        assert_eq!(cyclotomic_eval(13, p).unwrap(), Uint::from_u128(8191));
        // Phi_14(2) = 2^6 - 2^5 + 2^3 - 2^2 + 1? No, Phi_14 is cyclotomic(14). Phi_14(x) = X^6 - X^5 + X^4 - X^3 + X^2 - X + 1
        // For x=2: 64 - 32 + 16 - 8 + 4 - 2 + 1 = 43
        assert_eq!(cyclotomic_eval(14, p).unwrap(), Uint::from_u128(43));
        // Phi_15(2) = X^8 - X^7 + X^5 - X^4 + X^3 - X + 1
        // For x=2: 256 - 128 + 32 - 16 + 8 - 2 + 1 = 151
        assert_eq!(cyclotomic_eval(15, p).unwrap(), Uint::from_u128(151));
    }

    #[test]
    #[should_panic(expected = "compute_sigma overflow")]
    fn test_compute_sigma_overflow_sentinel() {
        setup();
        // Trigger the FFI overflow sentinel by requesting sigma of a value that exceeds 256-bits.
        // u64::MAX ^ 100 heavily exceeds 256 bits, triggering the FFI None sentinel.
        let p = u64::MAX;
        let pow = 100;
        let _ = compute_sigma(p, pow);
    }
}

use ualbf_macros::lean_ffi_export;
#[lean_ffi_export]
pub fn rust_dummy_macro_test(a: Uint, b: Uint) -> Uint {
    a + b
}

pub fn get_pollard_rho_iteration_limit() -> u32 {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_pollard_rho_iteration_limit;
        if let Err(e) = check_verified_bit(val as u64, 31, "get_pollard_rho_iteration_limit") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 31);
        unmasked as u32
    }
}

pub fn get_target_min_log10() -> u32 {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_target_min_log10;
        if let Err(e) = check_verified_bit(val as u64, 31, "get_target_min_log10") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 31);
        unmasked as u32
    }
}
pub fn get_target_max_log10() -> u32 {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_target_max_log10;
        if let Err(e) = check_verified_bit(val as u64, 31, "get_target_max_log10") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 31);
        unmasked as u32
    }
}
pub fn get_sieve_limit() -> usize {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_sieve_limit;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_sieve_limit") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        match check_platform_limit(unmasked as u64) {
            Ok(v) => v,
            Err(e) => {
                handle_verified_bit_err(e);
                usize::MAX
            }
        }
    }
}
pub fn get_max_exponent() -> u32 {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_max_exponent;
        if let Err(e) = check_verified_bit(val as u64, 31, "get_max_exponent") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 31);
        unmasked as u32
    }
}
pub fn get_prefix_stop_threshold() -> u64 {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_prefix_stop_threshold;
        if let Err(e) = check_verified_bit(val as u64, 63, "get_prefix_stop_threshold") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 63);
        unmasked as u64
    }
}
pub fn get_raycast_gpu_threshold() -> usize {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_raycast_gpu_threshold;
        if let Err(e) = check_verified_bit(val as u64, 31, "get_raycast_gpu_threshold") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 31);
        match check_platform_limit(unmasked as u64) {
            Ok(v) => v,
            Err(e) => {
                handle_verified_bit_err(e);
                usize::MAX
            }
        }
    }
}
pub fn get_raycast_chunk_size() -> usize {
    initialize_lean_runtime();
    unsafe {
        let val = ualbf_raycast_chunk_size;
        if let Err(e) = check_verified_bit(val as u64, 31, "get_raycast_chunk_size") {
            handle_verified_bit_err(e);
        }
        let unmasked = val & !(1 << 31);
        match check_platform_limit(unmasked as u64) {
            Ok(v) => v,
            Err(e) => {
                handle_verified_bit_err(e);
                usize::MAX
            }
        }
    }
}
