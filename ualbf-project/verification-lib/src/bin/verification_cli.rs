#[cfg(feature = "signing")]
use std::env;
#[cfg(feature = "signing")]
use std::path::PathBuf;
#[cfg(feature = "signing")]
use verification_lib::{
    compute_verified_core_hash_runtime, compute_verified_extension_hash_runtime, format_payload,
    verify_signature,
};

#[cfg(feature = "signing")]
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: verification_cli <command> [args...]");
        std::process::exit(1);
    }

    let command = &args[1];
    match command.as_str() {
        "hash-tcb" => {
            if args.len() < 3 || args.len() > 4 {
                eprintln!("Usage: verification_cli hash-tcb <repo_root> [--core|--extension]");
                std::process::exit(1);
            }
            let repo_root = PathBuf::from(&args[2]);
            let target = if args.len() == 4 {
                args[3].as_str()
            } else {
                "--core"
            };
            let hash_res = match target {
                "--core" => compute_verified_core_hash_runtime(&repo_root),
                "--extension" => compute_verified_extension_hash_runtime(&repo_root),
                _ => {
                    eprintln!("Invalid flag. Use --core or --extension.");
                    std::process::exit(1);
                }
            };

            match hash_res {
                Ok(hash) => println!("{}", hash),
                Err(e) => {
                    eprintln!("Error computing TCB hash: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "format-payload" => {
            if args.len() < 10 || args.len() > 12 {
                eprintln!("Usage: verification_cli format-payload <manifest_hash> <logic_hash> <extension_hash_or_none> <branches> <min_log10> <max_log10> <trace_hash> <factorization_depth> [sampling_rate] [deterministic_seed]");
                std::process::exit(1);
            }
            let ext_hash = if args[4] == "none" {
                None
            } else {
                Some(args[4].as_str())
            };

            let sampling_rate = if args.len() >= 11 && args[10] != "none" {
                Some(args[10].parse().unwrap())
            } else {
                None
            };
            let deterministic_seed = if args.len() >= 12 && args[11] != "none" {
                Some(args[11].parse().unwrap())
            } else {
                None
            };
            let payload = format_payload(
                &args[2],
                &args[3],
                ext_hash,
                args[5].parse().unwrap(),
                args[6].parse().unwrap(),
                args[7].parse().unwrap(),
                &args[8],
                args[9].parse().unwrap(),
                sampling_rate,
                deterministic_seed,
                None,
                None,
                None,
            );
            println!("{}", payload);
        }
        "verify-signature" => {
            if args.len() != 5 {
                eprintln!(
                    "Usage: verification_cli verify-signature <pubkey_hex> <sig_hex> <payload>"
                );
                std::process::exit(1);
            }
            match verify_signature(&args[2], &args[3], &args[4]) {
                Ok(true) => {
                    println!("true");
                    std::process::exit(0);
                }
                Ok(false) => {
                    println!("false");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error verifying signature: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "signing"))]
fn main() {
    eprintln!("CLI requires the 'signing' feature.");
    std::process::exit(1);
}
