use crate::types::UintExt;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GpuBloomWitness {
    pub p: u64,
    pub two_e: u32,
    pub is_obstructed: bool,
    pub obstructing_modulus: u32,
    pub residues: Vec<u32>,
    pub bloom_indices: Vec<u64>,
}

static GPU_WITNESSES: Mutex<Vec<GpuBloomWitness>> = Mutex::new(Vec::new());

pub fn clear_gpu_witnesses() {
    if let Ok(mut lock) = GPU_WITNESSES.lock() {
        lock.clear();
    }
}

pub fn get_gpu_witnesses() -> Vec<GpuBloomWitness> {
    if let Ok(lock) = GPU_WITNESSES.lock() {
        lock.clone()
    } else {
        Vec::new()
    }
}

pub fn add_gpu_witness(witness: GpuBloomWitness) {
    if let Ok(mut lock) = GPU_WITNESSES.lock() {
        lock.push(witness);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct GpuBloomWitnessRaw {
    pub p: u64,
    pub two_e: u32,
    pub is_obstructed: u32,
    pub obstructing_modulus: u32,
    pub residues: [u32; 4],
    pub bloom_indices: [u64; 4],
}

pub fn verify_gpu_witness_record(witness: &GpuBloomWitness) -> Result<(), String> {
    if witness.p <= 1 {
        return Err(format!("Invalid prime parameter p = {}", witness.p));
    }
    if witness.two_e < 2 || witness.two_e % 2 != 0 {
        return Err(format!("Invalid exponent two_e = {}", witness.two_e));
    }
    if witness.residues.len() != 4 {
        return Err(format!(
            "Expected 4 residues mod {{3, 5, 7, 11}}, got {}",
            witness.residues.len()
        ));
    }

    let moduli = [3u32, 5u32, 7u32, 11u32];
    let mut expected_residues = Vec::with_capacity(4);
    let mut expected_is_obstructed = false;
    let mut expected_obstructing_modulus = 0u32;

    for (m, &q) in moduli.iter().enumerate() {
        let mut sum = 0u32;
        let mut term = 1u32;
        let p_mod = (witness.p % q as u64) as u32;
        for _ in 0..=witness.two_e {
            sum = (sum + term) % q;
            term = (term * p_mod) % q;
        }
        expected_residues.push(sum);
        if sum == 0 && !expected_is_obstructed {
            expected_is_obstructed = true;
            expected_obstructing_modulus = q;
        }

        if witness.residues[m] != sum {
            return Err(format!(
                "Residue mismatch for component (p={}, 2e={}) mod {}: expected {}, got {}",
                witness.p, witness.two_e, q, sum, witness.residues[m]
            ));
        }
    }

    if witness.is_obstructed != expected_is_obstructed {
        return Err(format!(
            "Obstruction status mismatch for component (p={}, 2e={}): expected {}, got {}",
            witness.p, witness.two_e, expected_is_obstructed, witness.is_obstructed
        ));
    }

    if witness.obstructing_modulus != expected_obstructing_modulus {
        return Err(format!(
            "Obstructing modulus mismatch for component (p={}, 2e={}): expected {}, got {}",
            witness.p, witness.two_e, expected_obstructing_modulus, witness.obstructing_modulus
        ));
    }

    // Call Lean 4 check_crt_1155 from UALBF.Engine.Mod1155Bridge via FFI
    let crt_1155_val = ((witness.residues[0] as u64 * 385 * 1)
        + (witness.residues[1] as u64 * 231 * 1)
        + (witness.residues[2] as u64 * 165 * 3)
        + (witness.residues[3] as u64 * 105 * 2))
        % 1155;
    let x_l_uint = crate::types::Uint::from_u64(crt_1155_val);
    let z_uint = crate::types::Uint::from_u64(witness.p % 1155);

    let crt_check = crate::lean_ffi::check_crt_1155(&z_uint, &x_l_uint);
    let p_mod_1155 = witness.p % 1155;
    let p2_mod_1155 = (p_mod_1155 * p_mod_1155) % 1155;
    let expected_crt_check = (p2_mod_1155 % 3 == crt_1155_val % 3)
        && (p2_mod_1155 % 5 == crt_1155_val % 5)
        && (p2_mod_1155 % 7 == crt_1155_val % 7)
        && (p2_mod_1155 % 11 == crt_1155_val % 11);

    if crt_check != expected_crt_check {
        return Err(format!(
            "Lean 4 CRT 1155 bridge check failed for component (p={}, 2e={})",
            witness.p, witness.two_e
        ));
    }

    if !witness.is_obstructed {
        if witness.bloom_indices.is_empty() {
            return Err(format!(
                "Unobstructed witness for component (p={}, 2e={}) missing Bloom filter indices",
                witness.p, witness.two_e
            ));
        }
    } else {
        for &idx in &witness.bloom_indices {
            if idx != 0 {
                return Err(format!(
                    "Obstructed witness for component (p={}, 2e={}) has non-zero Bloom index {}",
                    witness.p, witness.two_e, idx
                ));
            }
        }
    }

    Ok(())
}

pub fn verify_gpu_witnesses_async(witnesses: &[GpuBloomWitness]) -> Result<(), String> {
    use rayon::prelude::*;
    let failure = witnesses.par_iter().find_map_any(|w| {
        if let Err(e) = verify_gpu_witness_record(w) {
            Some((w.clone(), e))
        } else {
            None
        }
    });

    if let Some((failed_witness, err_msg)) = failure {
        eprintln!(
            "GPU|ERROR|Calculation flagged in TCB.md and telemetry logs due to invalid GPU witness: p={}, two_e={}. Error: {}",
            failed_witness.p, failed_witness.two_e, err_msg
        );
        println!(
            "PROGRESS|TELEMETRY|GPU witness verification failure: p={}, two_e={}. Execution halted.",
            failed_witness.p, failed_witness.two_e
        );
        return Err(format!(
            "GPU witness verification failed for p={}, two_e={}: {}. Execution halted.",
            failed_witness.p, failed_witness.two_e, err_msg
        ));
    }

    Ok(())
}

pub fn get_component_hashes(p: u64, two_e: u32) -> (u64, u64) {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&p.to_be_bytes());
    hasher.update(&two_e.to_be_bytes());
    let result = hasher.finalize();
    let hash1 = u64::from_be_bytes(result[0..8].try_into().unwrap());
    let hash2 = u64::from_be_bytes(result[8..16].try_into().unwrap());
    (hash1, hash2)
}

#[deprecated(
    note = "Unverified GPU sieve routines are deprecated; engine uses pure Lean-verified CPU execution."
)]
pub fn run_gpu_sieve_and_generate_witnesses(
    components: &[crate::types::PrimePower],
    num_bits: u64,
    num_hashes: u32,
) -> Result<Vec<u32>, String> {
    clear_gpu_witnesses();
    let word_count = ((num_bits + 31) / 32) as usize;

    println!(
        "CPU|INFO|Executing parallel CPU Rayon-accelerated CRT Tensor Sieve & Bloom filter..."
    );

    use std::sync::Arc;
    let bitmap_atomics: Arc<Vec<std::sync::atomic::AtomicU32>> = Arc::new(
        (0..word_count)
            .map(|_| std::sync::atomic::AtomicU32::new(0))
            .collect(),
    );

    use rayon::prelude::*;
    let witnesses: Vec<GpuBloomWitness> = components
        .par_iter()
        .map(|comp| {
            let (hash1, hash2) = get_component_hashes(comp.p, comp.two_e);

            let moduli = [3, 5, 7, 11];
            let mut residues = Vec::new();
            let mut obstructing_modulus = 0;
            let mut is_obstructed = false;

            for &q in &moduli {
                let mut sum = 0u32;
                let mut term = 1u32;
                let p_mod = (comp.p % q as u64) as u32;
                for _ in 0..=comp.two_e {
                    sum = (sum + term) % q;
                    term = (term * p_mod) % q;
                }
                residues.push(sum);
                if sum == 0 && !is_obstructed {
                    is_obstructed = true;
                    obstructing_modulus = q;
                }
            }

            let mut bloom_indices = Vec::new();
            if !is_obstructed {
                for i in 0..num_hashes {
                    let cur = hash1
                        .wrapping_add((i as u64).wrapping_mul(hash2))
                        .wrapping_add(((i as u64).wrapping_mul((i as u64).wrapping_sub(1))) / 2);
                    let max_bits = if num_bits == 0 { 1 } else { num_bits };
                    let bit_idx = cur % max_bits;
                    bloom_indices.push(bit_idx);

                    let word_idx = (bit_idx / 32) as usize;
                    let bit_mask = 1u32 << (bit_idx % 32);
                    if word_idx < word_count {
                        bitmap_atomics[word_idx]
                            .fetch_or(bit_mask, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            GpuBloomWitness {
                p: comp.p,
                two_e: comp.two_e,
                is_obstructed,
                obstructing_modulus,
                residues,
                bloom_indices,
            }
        })
        .collect();

    verify_gpu_witnesses_async(&witnesses)?;

    for w in witnesses {
        add_gpu_witness(w);
    }

    let mut final_bitmap = vec![0u32; word_count];
    for i in 0..word_count {
        final_bitmap[i] = bitmap_atomics[i].load(std::sync::atomic::Ordering::Relaxed);
    }

    println!(
        "CPU|SUCCESS|CRT Tensor Sieve completed. Generated {} mathematical witnesses.",
        components.len()
    );
    Ok(final_bitmap)
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrtInputComponent {
    pub p: u64,
    pub two_e: u32,
    pub _padding: u32,
    pub hash1: u64,
    pub hash2: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_component(p: u64, two_e: u32) -> crate::types::PrimePower {
        crate::types::PrimePower {
            p,
            two_e,
            val: crate::types::Uint::zero(),
            sigma: crate::types::Uint::zero(),
            sigma_factors: vec![],
            needs_rho: vec![],
            abundance_fp: 0,
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_gpu_witness_verification_valid_records() {
        let components = vec![
            make_test_component(3, 2),
            make_test_component(5, 2),
            make_test_component(7, 2),
        ];
        let res = run_gpu_sieve_and_generate_witnesses(&components, 1024, 4);
        assert!(res.is_ok());
        let witnesses = get_gpu_witnesses();
        assert_eq!(witnesses.len(), 3);
        assert!(verify_gpu_witnesses_async(&witnesses).is_ok());
    }

    #[test]
    fn test_gpu_witness_verification_invalid_residue() {
        let mut witnesses = vec![GpuBloomWitness {
            p: 3,
            two_e: 2,
            is_obstructed: true,
            obstructing_modulus: 3,
            residues: vec![0, 0, 0, 0], // Incorrect residue mod 5, 7, 11
            bloom_indices: vec![],
        }];

        // Correct residues for (p=3, 2e=2) mod 3, 5, 7, 11 are [1, 3, 6, 2]
        // 13 mod 3 = 1, 13 mod 5 = 3, 13 mod 7 = 6, 13 mod 11 = 2
        witnesses[0].residues = vec![99, 3, 6, 2]; // Tampered residue mod 3
        let res = verify_gpu_witnesses_async(&witnesses);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Residue mismatch"));
    }

    #[test]
    fn test_gpu_witness_verification_tampered_obstruction() {
        let witnesses = vec![GpuBloomWitness {
            p: 3,
            two_e: 2,
            is_obstructed: false, // Tampered: (3, 2) is actually unobstructed since residues are [1, 3, 6, 2], so status matches, but let's test wrong obstructing modulus
            obstructing_modulus: 5, // Tampered: should be 0
            residues: vec![1, 3, 6, 2],
            bloom_indices: vec![10, 20, 30, 40],
        }];

        let res = verify_gpu_witnesses_async(&witnesses);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Obstructing modulus mismatch"));
    }

    #[test]
    fn test_gpu_witness_verification_tampered_bloom_indices() {
        let witnesses = vec![GpuBloomWitness {
            p: 3,
            two_e: 2,
            is_obstructed: true, // Tampered: claim obstructed when residues are [1, 3, 6, 2] none of which is 0
            obstructing_modulus: 3,
            residues: vec![1, 3, 6, 2],
            bloom_indices: vec![],
        }];

        let res = verify_gpu_witnesses_async(&witnesses);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Obstruction status mismatch"));
    }
}
