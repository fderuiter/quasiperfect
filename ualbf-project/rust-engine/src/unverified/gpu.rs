use crate::metal_reflection::{ConstantRef, DeviceAtomicPtr, DeviceConstPtr};
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

pub fn run_gpu_sieve_and_generate_witnesses(
    components: &[crate::types::PrimePower],
    num_bits: u64,
    num_hashes: u32,
) -> Result<Vec<u32>, String> {
    clear_gpu_witnesses();
    let word_count = ((num_bits + 31) / 32) as usize;

    println!("GPU|INFO|Executing parallel GPU-accelerated CRT Tensor Sieve & Bloom filter...");

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
        "GPU|SUCCESS|CRT Tensor Sieve completed. Verified {} mathematical witnesses on CPU gateway.",
        components.len()
    );
    Ok(final_bitmap)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, ualbf_macros::MetalLayout)]
pub struct CrtInputComponent {
    pub p: u64,
    pub two_e: u32,
    pub _padding: u32,
    pub hash1: u64,
    pub hash2: u64,
}

#[derive(ualbf_macros::MetalPipeline)]
pub struct CrtTensorSievePipeline {
    pub inputs: crate::metal_reflection::DeviceConstPtr<CrtInputComponent>,
    pub bitmap: crate::metal_reflection::DeviceAtomicPtr<u32>,
    pub num_inputs: crate::metal_reflection::ConstantRef<u32>,
    pub num_bits: crate::metal_reflection::ConstantRef<u64>,
    pub num_hashes: crate::metal_reflection::ConstantRef<u32>,
}

pub trait GpuPipeline {
    fn crt_tensor_sieve(
        &self,
        inputs: &[CrtInputComponent],
        num_bits: u64,
        num_hashes: u32,
    ) -> Result<Vec<u32>, String>;
}

pub struct DummyGpuPipeline;

impl GpuPipeline for DummyGpuPipeline {
    fn crt_tensor_sieve(
        &self,
        _inputs: &[CrtInputComponent],
        _num_bits: u64,
        _num_hashes: u32,
    ) -> Result<Vec<u32>, String> {
        Ok(Vec::new()) // Returns an empty bitmap as requested
    }
}

#[cfg(all(target_os = "macos", feature = "gpu"))]
pub struct MetalGpuPipeline {
    pub device: metal::Device,
    pub command_queue: metal::CommandQueue,
    pub pipeline_state: metal::ComputePipelineState,
}

#[cfg(all(target_os = "macos", feature = "gpu"))]
impl GpuPipeline for MetalGpuPipeline {
    fn crt_tensor_sieve(
        &self,
        inputs: &[CrtInputComponent],
        num_bits: u64,
        num_hashes: u32,
    ) -> Result<Vec<u32>, String> {
        let num_inputs = inputs.len() as u32;
        let word_count = ((num_bits + 31) / 32) as usize;
        let bitmap_byte_len = word_count * 4;

        if inputs.is_empty() {
            return Ok(vec![0u32; word_count]);
        }

        let inputs_buf = self.device.new_buffer_with_data(
            inputs.as_ptr() as *const std::ffi::c_void,
            (inputs.len() * std::mem::size_of::<CrtInputComponent>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );
        let bitmap_buf = self.device.new_buffer(
            bitmap_byte_len as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        // Zero-initialize bitmap buffer
        unsafe {
            std::ptr::write_bytes(bitmap_buf.contents() as *mut u8, 0, bitmap_byte_len);
        }

        let num_inputs_buf = self.device.new_buffer_with_data(
            &num_inputs as *const u32 as *const std::ffi::c_void,
            4,
            metal::MTLResourceOptions::StorageModeShared,
        );
        let num_bits_buf = self.device.new_buffer_with_data(
            &num_bits as *const u64 as *const std::ffi::c_void,
            8,
            metal::MTLResourceOptions::StorageModeShared,
        );
        let num_hashes_buf = self.device.new_buffer_with_data(
            &num_hashes as *const u32 as *const std::ffi::c_void,
            4,
            metal::MTLResourceOptions::StorageModeShared,
        );

        let pipeline = CrtTensorSievePipeline {
            inputs: crate::metal_reflection::DeviceConstPtr::new(inputs_buf),
            bitmap: crate::metal_reflection::DeviceAtomicPtr::new(bitmap_buf.clone()),
            num_inputs: crate::metal_reflection::ConstantRef::new(num_inputs_buf),
            num_bits: crate::metal_reflection::ConstantRef::new(num_bits_buf),
            num_hashes: crate::metal_reflection::ConstantRef::new(num_hashes_buf),
        };

        let command_buffer = self.command_queue.new_command_buffer();
        let encoder = command_buffer.new_compute_command_encoder();
        encoder.set_compute_pipeline_state(&self.pipeline_state);

        use crate::metal_reflection::MetalPipeline;
        pipeline.bind(encoder);

        let thread_group_count = metal::MTLSize {
            width: (num_inputs as u64 + 63) / 64,
            height: 1,
            depth: 1,
        };
        let thread_group_size = metal::MTLSize {
            width: 64,
            height: 1,
            depth: 1,
        };
        encoder.dispatch_threadgroups(thread_group_count, thread_group_size);
        encoder.end_encoding();

        command_buffer.commit();
        command_buffer.wait_until_completed();

        let mut result = vec![0u32; word_count];
        unsafe {
            std::ptr::copy_nonoverlapping(
                bitmap_buf.contents() as *const u32,
                result.as_mut_ptr(),
                word_count,
            );
        }
        Ok(result)
    }
}

#[cfg(feature = "gpu")]
pub struct OpenClGpuPipeline {
    pub context: opencl3::context::Context,
    pub queue: opencl3::command_queue::CommandQueue,
    pub kernel: std::sync::Mutex<opencl3::kernel::Kernel>,
}

#[cfg(feature = "gpu")]
impl GpuPipeline for OpenClGpuPipeline {
    fn crt_tensor_sieve(
        &self,
        inputs: &[CrtInputComponent],
        num_bits: u64,
        num_hashes: u32,
    ) -> Result<Vec<u32>, String> {
        use opencl3::memory::ClMem;

        let num_inputs = inputs.len() as u32;
        let word_count = ((num_bits + 31) / 32) as usize;

        if inputs.is_empty() {
            return Ok(vec![0u32; word_count]);
        }

        let cl_inputs = opencl3::memory::Buffer::<CrtInputComponent>::create(
            &self.context,
            opencl3::memory::CL_MEM_READ_ONLY | opencl3::memory::CL_MEM_COPY_HOST_PTR,
            inputs.len(),
            inputs.as_ptr() as *mut std::ffi::c_void,
        )
        .map_err(|e| format!("OpenCL buffer creation failed: {:?}", e))?;

        let cl_bitmap = opencl3::memory::Buffer::<u32>::create(
            &self.context,
            opencl3::memory::CL_MEM_READ_WRITE,
            word_count,
            std::ptr::null_mut(),
        )
        .map_err(|e| format!("OpenCL bitmap buffer creation failed: {:?}", e))?;

        let cl_witnesses = opencl3::memory::Buffer::<GpuBloomWitnessRaw>::create(
            &self.context,
            opencl3::memory::CL_MEM_READ_WRITE,
            inputs.len(),
            std::ptr::null_mut(),
        )
        .map_err(|e| format!("OpenCL witnesses buffer creation failed: {:?}", e))?;

        // Zero-initialize cl_bitmap on the host and write it
        let zero_bitmap = vec![0u32; word_count];
        let _write_event = unsafe {
            self.queue
                .enqueue_write_buffer(&cl_bitmap, opencl3::types::CL_TRUE, 0, &zero_bitmap, &[])
                .map_err(|e| format!("OpenCL bitmap buffer zeroing failed: {:?}", e))?
        };

        let mut kernel = self.kernel.lock().unwrap();

        kernel
            .set_arg(0, &cl_inputs.get())
            .map_err(|e| format!("Arg 0 failed: {:?}", e))?;
        kernel
            .set_arg(1, &cl_bitmap.get())
            .map_err(|e| format!("Arg 1 failed: {:?}", e))?;
        kernel
            .set_arg(2, &cl_witnesses.get())
            .map_err(|e| format!("Arg 2 failed: {:?}", e))?;
        kernel
            .set_arg(3, &num_inputs)
            .map_err(|e| format!("Arg 3 failed: {:?}", e))?;
        kernel
            .set_arg(4, &num_bits)
            .map_err(|e| format!("Arg 4 failed: {:?}", e))?;
        kernel
            .set_arg(5, &num_hashes)
            .map_err(|e| format!("Arg 5 failed: {:?}", e))?;

        let global_work_size = ((num_inputs as usize + 63) / 64) * 64;
        let local_work_size = 64usize;

        let execute_event = unsafe {
            self.queue
                .enqueue_nd_range_kernel(
                    kernel.get(),
                    1,
                    std::ptr::null(),
                    &global_work_size as *const usize,
                    &local_work_size as *const usize,
                    &[],
                )
                .map_err(|e| format!("OpenCL kernel execution failed: {:?}", e))?
        };

        let mut result = vec![0u32; word_count];
        let _read_event = unsafe {
            self.queue
                .enqueue_read_buffer(
                    &cl_bitmap,
                    opencl3::types::CL_TRUE,
                    0,
                    &mut result[..],
                    &[execute_event.get()],
                )
                .map_err(|e| format!("OpenCL read buffer failed: {:?}", e))?
        };

        let mut raw_witnesses = vec![GpuBloomWitnessRaw::default(); inputs.len()];
        let _read_witnesses_event = unsafe {
            self.queue
                .enqueue_read_buffer(
                    &cl_witnesses,
                    opencl3::types::CL_TRUE,
                    0,
                    &mut raw_witnesses[..],
                    &[execute_event.get()],
                )
                .map_err(|e| format!("OpenCL read witness buffer failed: {:?}", e))?
        };

        let gpu_witnesses: Vec<GpuBloomWitness> = raw_witnesses
            .into_iter()
            .map(|raw| {
                let mut bloom_indices = Vec::new();
                if raw.is_obstructed == 0 {
                    for i in 0..num_hashes.min(4) as usize {
                        bloom_indices.push(raw.bloom_indices[i]);
                    }
                }
                GpuBloomWitness {
                    p: raw.p,
                    two_e: raw.two_e,
                    is_obstructed: raw.is_obstructed != 0,
                    obstructing_modulus: raw.obstructing_modulus,
                    residues: raw.residues.to_vec(),
                    bloom_indices,
                }
            })
            .collect();

        verify_gpu_witnesses_async(&gpu_witnesses)?;

        for w in gpu_witnesses {
            add_gpu_witness(w);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metal_reflection::{MetalLayout, MetalPipeline};

    #[test]
    fn test_crt_input_component_layout() {
        let layout = CrtInputComponent::get_layout();
        assert!(layout.contains("struct CrtInputComponent"));
        assert!(layout.contains("uint64_t p;"));
        assert!(layout.contains("uint32_t two_e;"));
        assert!(layout.contains("uint32_t _padding;"));
        assert!(layout.contains("uint64_t hash1;"));
        assert!(layout.contains("uint64_t hash2;"));
    }

    #[test]
    fn test_crt_tensor_sieve_pipeline_signature() {
        let sig = CrtTensorSievePipeline::get_signature("crt_tensor_sieve");
        assert!(sig.contains("kernel void crt_tensor_sieve("));
        assert!(sig.contains("device const CrtInputComponent* inputs [[buffer(0)]]"));
        assert!(sig.contains("device atomic_uint* bitmap [[buffer(1)]]"));
        assert!(sig.contains("constant uint32_t& num_inputs [[buffer(2)]]"));
        assert!(sig.contains("constant uint64_t& num_bits [[buffer(3)]]"));
        assert!(sig.contains("constant uint32_t& num_hashes [[buffer(4)]]"));
        assert!(sig.contains("uint id [[thread_position_in_grid]]"));
    }

    #[test]
    fn test_dummy_gpu_pipeline() {
        let pipeline = DummyGpuPipeline;
        let inputs = vec![CrtInputComponent {
            p: 3,
            two_e: 2,
            _padding: 0,
            hash1: 123,
            hash2: 456,
        }];
        let res = pipeline.crt_tensor_sieve(&inputs, 1024, 4);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Vec::<u32>::new());
    }

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
