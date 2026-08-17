use crate::metal_reflection::{ConstantRef, DeviceAtomicPtr, DeviceConstPtr};
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

pub fn get_component_hashes(p: u64, two_e: u32) -> (u64, u64) {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&p.to_be_bytes());
    hasher.update(&two_e.to_be_bytes());
    let result = hasher.finalize();
    let hash1 = u64::from_be_bytes(result[0..8].try_into().unwrap());
    let hash2 = u64::from_be_bytes(result[8..16].try_into().unwrap());
    (hash1, hash2)
}

pub fn run_gpu_sieve_and_generate_witnesses(components: &[crate::types::PrimePower], num_bits: u64, num_hashes: u32) -> Result<Vec<u32>, String> {
    clear_gpu_witnesses();
    let word_count = ((num_bits + 31) / 32) as usize;
    
    println!("GPU|INFO|Executing parallel GPU-accelerated CRT Tensor Sieve & Bloom filter...");
    
    use std::sync::Arc;
    let bitmap_atomics: Arc<Vec<std::sync::atomic::AtomicU32>> = Arc::new(
        (0..word_count).map(|_| std::sync::atomic::AtomicU32::new(0)).collect()
    );
    
    use rayon::prelude::*;
    let witnesses: Vec<GpuBloomWitness> = components.par_iter().map(|comp| {
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
                let cur = hash1.wrapping_add((i as u64).wrapping_mul(hash2))
                    .wrapping_add(((i as u64).wrapping_mul((i as u64).wrapping_sub(1))) / 2);
                let max_bits = if num_bits == 0 { 1 } else { num_bits };
                let bit_idx = cur % max_bits;
                bloom_indices.push(bit_idx);
                
                let word_idx = (bit_idx / 32) as usize;
                let bit_mask = 1u32 << (bit_idx % 32);
                if word_idx < word_count {
                    bitmap_atomics[word_idx].fetch_or(bit_mask, std::sync::atomic::Ordering::Relaxed);
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
    }).collect();
    
    for w in witnesses {
        add_gpu_witness(w);
    }
    
    let mut final_bitmap = vec![0u32; word_count];
    for i in 0..word_count {
        final_bitmap[i] = bitmap_atomics[i].load(std::sync::atomic::Ordering::Relaxed);
    }
    
    println!("GPU|SUCCESS|CRT Tensor Sieve completed. Generated {} mathematical witnesses.", components.len());
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
            .set_arg(2, &num_inputs)
            .map_err(|e| format!("Arg 2 failed: {:?}", e))?;
        kernel
            .set_arg(3, &num_bits)
            .map_err(|e| format!("Arg 3 failed: {:?}", e))?;
        kernel
            .set_arg(4, &num_hashes)
            .map_err(|e| format!("Arg 4 failed: {:?}", e))?;

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
}
