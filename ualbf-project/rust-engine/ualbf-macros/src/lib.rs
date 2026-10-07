extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Pat, PatIdent, PatType, ReturnType};

#[proc_macro_attribute]
pub fn lean_ffi_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let block = &input_fn.block;

    let mut args = Vec::new();
    let mut arg_wrappers = Vec::new();
    let mut arg_vars = Vec::new();

    for arg in &input_fn.sig.inputs {
        if let syn::FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                args.push(quote! { #ident: *mut crate::lean_ffi::lean_object });
                arg_wrappers.push(quote! {
                    let #ident = crate::lean_ffi::LeanObjectWrapper::new(#ident);
                });
                arg_vars.push(quote! {
                    let #ident: #ty = match crate::lean_ffi::TryFromLean::try_from_lean(#ident.as_ptr()) {
                        Ok(v) => v,
                        Err(_) => return None,
                    };
                });
            }
        }
    }

    let output = match &input_fn.sig.output {
        ReturnType::Default => {
            quote! {
                #[no_mangle]
                pub extern "C" fn #fn_name(#(#args),*) {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #(#arg_wrappers)*
                        let body = || -> Option<()> {
                            #(#arg_vars)*
                            #block;
                            Some(())
                        };
                        body();
                    }));
                }
            }
        }
        ReturnType::Type(_, ty) => {
            quote! {
                #[no_mangle]
                pub extern "C" fn #fn_name(#(#args),*) -> *mut crate::lean_ffi::lean_object {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #(#arg_wrappers)*
                        let body = || -> Option<*mut crate::lean_ffi::lean_object> {
                            #(#arg_vars)*
                            let result: #ty = #block;
                            Some(crate::lean_ffi::ToLean::to_lean(&result).into_raw())
                        };
                        body().unwrap_or(std::ptr::null_mut())
                    })).unwrap_or(std::ptr::null_mut())
                }
            }
        }
    };

    output.into()
}

#[proc_macro]
pub fn universal_pruning_bounds(_input: TokenStream) -> TokenStream {
    let metal_code = r#"
inline RNS512 ualbf_mul_u64(RNS512 a, uint64_t b) {
    RNS512 res;
    uint64_t carry = 0;
    for(int i=0; i<8; i++) {
        uint64_t lo = a.w[i] * b;
        uint64_t hi = mulhi(a.w[i], b);
        uint64_t sum1 = lo + carry;
        uint64_t c1 = (sum1 < carry) ? 1 : 0;
        res.w[i] = sum1;
        carry = hi + c1;
    }
    return res;
}

inline bool ualbf_check_abundancy_overflow(RNS512 s_l, RNS512 n_l, uint64_t overflow_den, uint64_t overflow_num) {
    RNS512 lhs = ualbf_mul_u64(s_l, overflow_den);
    RNS512 rhs = ualbf_mul_u64(n_l, overflow_num);
    return cmp(lhs, rhs) > 0;
}

inline bool ualbf_check_euler_ceiling(RNS512 num, RNS512 den, uint64_t euler_num, uint64_t euler_den) {
    RNS512 lhs = ualbf_mul_u64(num, euler_den);
    RNS512 rhs = ualbf_mul_u64(den, euler_num);
    return cmp(lhs, rhs) < 0;
}

inline bool ualbf_check_prasad_sunitha(uint32_t info_mask, uint32_t baseline_min, uint32_t prasad_sunitha_bound, uint32_t curr_factors_len, uint32_t remaining_components) {
    uint32_t dynamic_min = baseline_min;
    if ((info_mask & 3) == 0 && (info_mask & 12) == 12) {
        dynamic_min = prasad_sunitha_bound;
    }
    uint32_t remaining_needed = 0;
    if (dynamic_min > curr_factors_len) {
        remaining_needed = dynamic_min - curr_factors_len;
    }
    if (remaining_needed > 0) {
        if (remaining_components < remaining_needed) {
            return true;
        }
    }
    return false;
}

inline bool ualbf_check_dusart_bound(RNS512 s_l, RNS512 n_l, uint64_t p_last, uint64_t validity_threshold, uint64_t dusart_num, uint64_t dusart_den, uint64_t target_num, uint64_t target_den) {
    if (p_last < validity_threshold) {
        return false;
    }
    uint64_t den_p_last = dusart_den * p_last;
    uint64_t factor_num = den_p_last + dusart_num;
    uint64_t factor_den = den_p_last;

    RNS512 lhs = ualbf_mul_u64(s_l, factor_num);
    lhs = ualbf_mul_u64(lhs, target_den);

    RNS512 rhs = ualbf_mul_u64(n_l, factor_den);
    rhs = ualbf_mul_u64(rhs, target_num);

    return cmp(lhs, rhs) < 0;
}
"#;

    let rust_code = quote! {
        use crate::types::UintExt;
        pub const METAL_PRUNING_LOGIC: &str = #metal_code;

        pub fn cpu_check_abundancy_overflow(s_l: &crate::types::Uint, n_l: &crate::types::Uint, target_num: u64, target_den: u64) -> bool {
            let num = crate::types::Uint::from_u64(target_num);
            let den = crate::types::Uint::from_u64(target_den);
            s_l * den > n_l * num
        }

        pub fn cpu_check_euler_ceiling(num: &crate::types::Uint, den: &crate::types::Uint, euler_num: &crate::types::Uint, euler_den: &crate::types::Uint) -> bool {
            let enum_u = euler_num;
            let eden_u = euler_den;
            let lhs = num.checked_mul(*eden_u);
            let rhs = den.checked_mul(*enum_u);
            match (lhs, rhs) {
                (Some(l), Some(r)) => l < r,
                _ => true,
            }
        }

        pub fn cpu_check_cdg_forced(s_l: &crate::types::Uint, n_l: &crate::types::Uint, forced_num: &crate::types::Uint, forced_den: &crate::types::Uint, target_num: u64, target_den: u64) -> bool {
            let num = crate::types::Uint::from_u64(target_num);
            let den = crate::types::Uint::from_u64(target_den);
            let lhs_opt = s_l.checked_mul(*forced_num).and_then(|x| x.checked_mul(den));
            let rhs_opt = n_l.checked_mul(*forced_den).and_then(|x| x.checked_mul(num));
            match (lhs_opt, rhs_opt) {
                (Some(lhs), Some(rhs)) => lhs > rhs,
                _ => false,
            }
        }

        pub fn cpu_check_prasad_sunitha(info_mask: u32, baseline_min: usize, prasad_sunitha_bound: usize, curr_factors_len: usize, remaining_components: usize) -> bool {
            let mut dynamic_min = baseline_min;
            if (info_mask & 3) == 0 && (info_mask & 12) == 12 {
                dynamic_min = prasad_sunitha_bound;
            }
            let remaining_needed = dynamic_min.saturating_sub(curr_factors_len);
            if remaining_needed > 0 {
                if remaining_components < remaining_needed {
                    return true;
                }
            }
            false
        }

        pub fn cpu_check_dusart_bound(
            s_l: &crate::types::Uint,
            n_l: &crate::types::Uint,
            p_last: u64,
            validity_threshold: u64,
            dusart_num: u64,
            dusart_den: u64,
            target_num: u64,
            target_den: u64,
        ) -> bool {
            if p_last < validity_threshold {
                return false;
            }
            let den_p_last = match dusart_den.checked_mul(p_last) {
                Some(v) => v,
                None => return false,
            };
            let factor_num = match den_p_last.checked_add(dusart_num) {
                Some(v) => v,
                None => return false,
            };
            let factor_den = den_p_last;

            let factor_num_u = crate::types::Uint::from_u64(factor_num);
            let factor_den_u = crate::types::Uint::from_u64(factor_den);
            let target_num_u = crate::types::Uint::from_u64(target_num);
            let target_den_u = crate::types::Uint::from_u64(target_den);

            let lhs_opt = s_l
                .checked_mul(factor_num_u)
                .and_then(|x| x.checked_mul(target_den_u));
            let rhs_opt = n_l
                .checked_mul(factor_den_u)
                .and_then(|x| x.checked_mul(target_num_u));

            match (lhs_opt, rhs_opt) {
                (Some(lhs), Some(rhs)) => lhs < rhs,
                _ => false,
            }
        }
    };

    rust_code.into()
}
