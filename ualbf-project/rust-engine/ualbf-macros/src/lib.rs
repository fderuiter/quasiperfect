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
    let rust_code = quote! {
        use crate::types::UintExt;

        pub fn cpu_check_abundancy_overflow(s_l: &crate::types::Uint, n_l: &crate::types::Uint, target_num: u64, target_den: u64) -> bool {
            let num = crate::types::Uint::from_u64(target_num);
            let den = crate::types::Uint::from_u64(target_den);
            s_l * den > n_l * num
        }

        pub fn cpu_check_euler_ceiling(num: &crate::types::Uint, den: &crate::types::Uint, euler_num: &crate::types::Uint, euler_den: &crate::types::Uint) -> bool {
            let enum_u = euler_num;
            let eden_u = euler_den;
            num * eden_u > den * enum_u
        }

        pub fn cpu_check_cdg_forced(s_l: &crate::types::Uint, n_l: &crate::types::Uint, forced_num: &crate::types::Uint, forced_den: &crate::types::Uint, target_num: u64, target_den: u64) -> bool {
            let num = crate::types::Uint::from_u64(target_num);
            let den = crate::types::Uint::from_u64(target_den);
            let lhs = s_l.checked_mul(*forced_num).and_then(|x| x.checked_mul(den)).unwrap_or(crate::types::Uint::MAX);
            let rhs = n_l.checked_mul(*forced_den).and_then(|x| x.checked_mul(num)).unwrap_or(crate::types::Uint::MAX);
            lhs > rhs
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
            let den_p_last = dusart_den.checked_mul(p_last).unwrap_or(u64::MAX);
            let factor_num = den_p_last.checked_add(dusart_num).unwrap_or(u64::MAX);
            let factor_den = den_p_last;

            let factor_num_u = crate::types::Uint::from_u64(factor_num);
            let factor_den_u = crate::types::Uint::from_u64(factor_den);
            let target_num_u = crate::types::Uint::from_u64(target_num);
            let target_den_u = crate::types::Uint::from_u64(target_den);

            let lhs = s_l.checked_mul(factor_num_u)
                .and_then(|x| x.checked_mul(target_den_u))
                .unwrap_or(crate::types::Uint::MAX);
            let rhs = n_l.checked_mul(factor_den_u)
                .and_then(|x| x.checked_mul(target_num_u))
                .unwrap_or(crate::types::Uint::MAX);

            lhs < rhs
        }
    };

    rust_code.into()
}
