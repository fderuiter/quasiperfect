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
                    let #ident: #ty = crate::lean_ffi::FromLean::from_lean(#ident.as_ptr());
                });
            }
        }
    }

    let output = match &input_fn.sig.output {
        ReturnType::Default => {
            quote! {
                #[no_mangle]
                pub extern "C" fn #fn_name(#(#args),*) {
                    #(#arg_wrappers)*
                    #(#arg_vars)*
                    #block
                }
            }
        }
        ReturnType::Type(_, ty) => {
            quote! {
                #[no_mangle]
                pub extern "C" fn #fn_name(#(#args),*) -> *mut crate::lean_ffi::lean_object {
                    #(#arg_wrappers)*
                    #(#arg_vars)*
                    let result: #ty = #block;
                    crate::lean_ffi::ToLean::to_lean(&result).into_raw()
                }
            }
        }
    };

    output.into()
}

#[proc_macro]
pub fn universal_pruning_bounds(_input: TokenStream) -> TokenStream {
    // Generate the Metal code for the bounds
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

inline bool ualbf_check_abundancy_overflow(RNS512 s_l, RNS512 n_l, uint64_t target_num, uint64_t target_den) {
    RNS512 lhs = ualbf_mul_u64(s_l, target_den);
    RNS512 rhs = ualbf_mul_u64(n_l, target_num);
    return cmp(lhs, rhs) > 0;
}

inline bool ualbf_check_euler_ceiling(RNS512 num, RNS512 den, uint64_t euler_num, uint64_t euler_den) {
    RNS512 lhs = ualbf_mul_u64(num, euler_den);
    RNS512 rhs = ualbf_mul_u64(den, euler_num);
    return cmp(lhs, rhs) > 0;
}

inline bool ualbf_check_cdg_forced(RNS512 s_l, RNS512 n_l, RNS512 forced_num, RNS512 forced_den, uint64_t target_num, uint64_t target_den) {
    RNS512 lhs = ualbf_mul_u64(s_l, target_den);
    RNS512 rhs = ualbf_mul_u64(n_l, target_num);
    return cmp(lhs, rhs) > 0;
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
            return true; // Prune!
        }
    }
    return false;
}
"#;

    // Generate the identical Rust code
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
    };

    rust_code.into()
}

#[proc_macro_derive(MetalLayout)]
pub fn metal_layout_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let mut fields_str = String::new();

    if let syn::Data::Struct(data_struct) = &input.data {
        if let syn::Fields::Named(fields) = &data_struct.fields {
            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let ty_str = match &field.ty {
                    syn::Type::Path(p) => p.path.segments.last().unwrap().ident.to_string(),
                    syn::Type::Array(a) => {
                        let inner_ty = if let syn::Type::Path(p) = &*a.elem {
                            p.path.segments.last().unwrap().ident.to_string()
                        } else {
                            "unknown".to_string()
                        };
                        let len = quote::quote!(#a).to_string();
                        // Extract just the number from `[u64 ; 8]` -> `8`
                        let len_val = len
                            .split(';')
                            .nth(1)
                            .unwrap_or("")
                            .trim()
                            .trim_end_matches(']')
                            .trim();
                        format!("{}[{}]", inner_ty, len_val)
                    }
                    _ => "unknown".to_string(),
                };

                // Map Rust types to Metal types
                let mut metal_ty = ty_str.clone();
                let mut array_suffix = String::new();

                if let Some(idx) = ty_str.find('[') {
                    metal_ty = ty_str[..idx].to_string();
                    array_suffix = ty_str[idx..].to_string();
                }

                let metal_base_ty = match metal_ty.as_str() {
                    "u64" => "uint64_t",
                    "u32" => "uint32_t",
                    "u8" => "uint8_t",
                    "bool" => "bool",
                    other => other,
                };

                fields_str.push_str(&format!(
                    "    {} {}{};\n",
                    metal_base_ty, field_name, array_suffix
                ));
            }
        }
    }

    let layout_str = format!("struct {} {{\n{}}};\n", name_str, fields_str);

    let output = quote::quote! {
        impl crate::metal_reflection::MetalLayout for #name {
            fn get_layout() -> String {
                #layout_str.to_string()
            }
        }
    };
    output.into()
}

#[proc_macro_derive(MetalPipeline)]
pub fn metal_pipeline_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;

    let mut bind_stmts = Vec::new();
    let mut sig_args = String::new();

    if let syn::Data::Struct(data_struct) = &input.data {
        if let syn::Fields::Named(fields) = &data_struct.fields {
            for (idx, field) in fields.named.iter().enumerate() {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();

                // Get the type e.g., DeviceConstRef<RNS512>
                let ty_str = if let syn::Type::Path(p) = &field.ty {
                    let seg = p.path.segments.last().unwrap();
                    let wrapper_name = seg.ident.to_string();
                    let inner_ty = if let syn::PathArguments::AngleBracketed(args) = &seg.arguments
                    {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(inner_p))) =
                            args.args.first()
                        {
                            inner_p.path.segments.last().unwrap().ident.to_string()
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    };
                    (wrapper_name, inner_ty)
                } else {
                    ("unknown".to_string(), "unknown".to_string())
                };

                let (wrapper, inner) = ty_str;
                let metal_inner = match inner.as_str() {
                    "u64" => "uint64_t",
                    "u32" => "uint32_t",
                    "u8" => "uint8_t",
                    "bool" => "bool",
                    other => other,
                };

                let access_modifier = match wrapper.as_str() {
                    "DeviceConstRef" => format!("device const {}&", metal_inner),
                    "ConstantRef" => format!("constant {}&", metal_inner),
                    "DeviceConstPtr" => format!("device const {}*", metal_inner),
                    "DevicePtr" => format!("device {}*", metal_inner),
                    "DeviceAtomicPtr" => format!("device atomic_{}*", metal_inner), // assuming atomic_uint etc. But if it's u32 it's atomic_uint
                    _ => format!("device {}&", metal_inner),
                };

                let access_modifier = if wrapper == "DeviceAtomicPtr" && inner == "u32" {
                    "device atomic_uint*".to_string()
                } else {
                    access_modifier
                };

                sig_args.push_str(&format!(
                    "    {} {} [[buffer({})]],\n",
                    access_modifier, field_name_str, idx
                ));

                let idx_lit = idx as u64;
                bind_stmts.push(quote::quote! {
                    encoder.set_buffer(#idx_lit, Some(&self.#field_name.0), 0);
                });
            }
        }
    }

    sig_args.push_str("    uint id [[thread_position_in_grid]]");

    let output = quote::quote! {
        impl crate::metal_reflection::MetalPipeline for #name {
            fn get_signature(kernel_name: &str) -> String {
                format!("kernel void {}(\n{}\n) {{", kernel_name, #sig_args)
            }

            #[cfg(target_os = "macos")]
            fn bind(&self, encoder: &metal::ComputeCommandEncoderRef) {
                #(#bind_stmts)*
            }
        }
    };
    output.into()
}
