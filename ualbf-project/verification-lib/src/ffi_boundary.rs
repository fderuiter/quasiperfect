use std::ffi::{CStr, FromBytesUntilNulError};
use std::ops::{Deref, DerefMut};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

/// Safe smart pointer wrapper for raw `*const T`.
/// Enforces non-null and alignment checks.
#[derive(Debug)]
pub struct FfiPtr<'a, T> {
    ptr: &'a T,
}

impl<'a, T> FfiPtr<'a, T> {
    #[inline(always)]
    pub fn new(raw: *const T) -> Option<Self> {
        let nn = NonNull::new(raw as *mut T)?;
        if !(nn.as_ptr() as usize).is_multiple_of(std::mem::align_of::<T>()) {
            return None;
        }
        unsafe { Some(Self { ptr: nn.as_ref() }) }
    }

    #[inline(always)]
    pub fn get_ref(&self) -> &'a T {
        self.ptr
    }

    /// Converts a bounded C string buffer starting at this pointer into a `&CStr`.
    ///
    /// Scans up to `max_len` bytes for a null terminator using C `memchr`.
    /// Returns `Err(FromBytesUntilNulError)` if no null byte is found within `max_len` bytes.
    pub fn to_cstr_bounded(&self, max_len: usize) -> Result<&'a CStr, FromBytesUntilNulError> {
        if max_len == 0 {
            return CStr::from_bytes_until_nul(&[]);
        }
        let raw_ptr = self.ptr as *const T as *const u8;
        let nul_ptr = unsafe {
            extern "C" {
                fn memchr(
                    s: *const std::ffi::c_void,
                    c: std::ffi::c_int,
                    n: usize,
                ) -> *mut std::ffi::c_void;
            }
            memchr(raw_ptr as *const std::ffi::c_void, 0, max_len)
        };
        if nul_ptr.is_null() {
            return CStr::from_bytes_until_nul(&[]);
        }
        let len = (nul_ptr as usize) - (raw_ptr as usize);
        let slice = unsafe { std::slice::from_raw_parts(raw_ptr, len + 1) };
        CStr::from_bytes_until_nul(slice)
    }
}

impl<'a, T> AsRef<T> for FfiPtr<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.ptr
    }
}

impl<'a, T> Deref for FfiPtr<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

/// Safe smart pointer wrapper for raw `*mut T`.
/// Enforces non-null and alignment checks.
#[derive(Debug)]
pub struct FfiMutPtr<'a, T> {
    ptr: &'a mut T,
}

impl<'a, T> FfiMutPtr<'a, T> {
    #[inline(always)]
    pub fn new(raw: *mut T) -> Option<Self> {
        let mut nn = NonNull::new(raw)?;
        if !(nn.as_ptr() as usize).is_multiple_of(std::mem::align_of::<T>()) {
            return None;
        }
        unsafe { Some(Self { ptr: nn.as_mut() }) }
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.ptr
    }

    #[inline(always)]
    pub fn get_ref(&self) -> &T {
        self.ptr
    }

    #[inline(always)]
    pub fn into_mut(self) -> &'a mut T {
        self.ptr
    }
}

impl<'a, T> AsRef<T> for FfiMutPtr<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.ptr
    }
}

impl<'a, T> AsMut<T> for FfiMutPtr<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        self.ptr
    }
}

impl<'a, T> Deref for FfiMutPtr<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a, T> DerefMut for FfiMutPtr<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr
    }
}

/// Type-safe context handle wrapper for handle conversions (e.g., `ctx: u64` or `*mut T`).
#[derive(Debug, Clone, Copy)]
pub struct NonNullContext<T> {
    raw: *mut T,
}

impl<T> NonNullContext<T> {
    #[inline(always)]
    pub fn from_u64(handle: u64) -> Option<Self> {
        if handle == 0 {
            return None;
        }
        let raw = handle as *mut T;
        if !(raw as usize).is_multiple_of(std::mem::align_of::<T>()) {
            return None;
        }
        Some(Self { raw })
    }

    #[inline(always)]
    pub fn new(raw: *mut T) -> Option<Self> {
        let nn = NonNull::new(raw)?;
        if !(nn.as_ptr() as usize).is_multiple_of(std::mem::align_of::<T>()) {
            return None;
        }
        Some(Self { raw: nn.as_ptr() })
    }

    /// # Safety
    /// The inner pointer must be valid for reading `'a`.
    #[inline(always)]
    pub unsafe fn as_ref<'a>(&self) -> Option<&'a T> {
        self.raw.as_ref()
    }

    /// # Safety
    /// The inner pointer must be valid for writing `'a`.
    #[inline(always)]
    pub unsafe fn as_mut<'a>(&self) -> Option<&'a mut T> {
        self.raw.as_mut()
    }
}

/// Intercepts panics across FFI boundaries and converts them to standard fallback values.
#[inline(always)]
pub fn catch_ffi_panic<F, R>(default: R, f: F) -> R
where
    F: FnOnce() -> R,
{
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| {
        eprintln!("Caught panic at FFI boundary");
        default
    })
}

#[macro_export]
macro_rules! safe_ffi_boundary {
    ($default:expr, $body:expr) => {
        $crate::ffi_boundary::catch_ffi_panic($default, || $body)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_ptr_valid_and_null() {
        let val: u64 = 42;
        let ptr = &val as *const u64;
        let ffi_ptr = FfiPtr::new(ptr);
        assert!(ffi_ptr.is_some());
        assert_eq!(*ffi_ptr.unwrap(), 42);

        let null_ptr: *const u64 = std::ptr::null();
        assert!(FfiPtr::new(null_ptr).is_none());
    }

    #[test]
    fn test_ffi_mut_ptr_valid_and_null() {
        let mut val: u64 = 42;
        let ptr = &mut val as *mut u64;
        if let Some(mut ffi_ptr) = FfiMutPtr::new(ptr) {
            *ffi_ptr = 100;
        }
        assert_eq!(val, 100);

        let null_ptr: *mut u64 = std::ptr::null_mut();
        assert!(FfiMutPtr::new(null_ptr).is_none());
    }

    #[test]
    fn test_non_null_context() {
        let mut val: u64 = 123;
        let handle = &mut val as *mut u64 as u64;
        let ctx = NonNullContext::<u64>::from_u64(handle).unwrap();
        unsafe {
            assert_eq!(*ctx.as_ref().unwrap(), 123);
        }

        assert!(NonNullContext::<u64>::from_u64(0).is_none());
    }

    #[test]
    fn test_catch_ffi_panic() {
        let res = catch_ffi_panic(0, || {
            panic!("Test panic");
        });
        assert_eq!(res, 0);

        let res_ok = catch_ffi_panic(10, || 42);
        assert_eq!(res_ok, 42);
    }

    #[test]
    fn test_to_cstr_bounded() {
        // Null-terminated string within limit succeeds
        let buf = b"hello\0world";
        let ffi_ptr = FfiPtr::new(buf.as_ptr() as *const std::ffi::c_char).unwrap();
        let cstr = ffi_ptr.to_cstr_bounded(10).unwrap();
        assert_eq!(cstr.to_str().unwrap(), "hello");

        // String without null byte within max_len fails
        let no_null = b"1234567890";
        let ffi_ptr2 = FfiPtr::new(no_null.as_ptr() as *const std::ffi::c_char).unwrap();
        assert!(ffi_ptr2.to_cstr_bounded(10).is_err());

        // Null byte at exactly max_len - 1 succeeds
        let exact = b"abc\0";
        let ffi_ptr3 = FfiPtr::new(exact.as_ptr() as *const std::ffi::c_char).unwrap();
        let cstr3 = ffi_ptr3.to_cstr_bounded(4).unwrap();
        assert_eq!(cstr3.to_str().unwrap(), "abc");

        // Null byte past max_len fails
        let late_null = b"abc\0";
        let ffi_ptr4 = FfiPtr::new(late_null.as_ptr() as *const std::ffi::c_char).unwrap();
        assert!(ffi_ptr4.to_cstr_bounded(3).is_err());
    }
}
