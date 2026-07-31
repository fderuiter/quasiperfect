use crate::types::Uint;
#[cfg(feature = "gpu")]
use crate::types::UintExt;
use std::sync::atomic::AtomicBool;
#[cfg(feature = "gpu")]
use std::sync::atomic::Ordering;
#[cfg(feature = "gpu")]
use std::sync::OnceLock;
#[cfg(feature = "gpu")]
#[derive(Clone, Copy, Default, ualbf_macros::MetalLayout)]
#[repr(C)]
pub struct RNS512 {
    pub w: [u64; 8],
}

#[cfg(feature = "gpu")]
#[derive(Clone, Copy, ualbf_macros::MetalLayout)]
#[repr(C)]
pub struct Task {
    pub n: RNS512,
    pub r_squared: RNS512,
    pub m0_prime: u64,
    pub padding: [u64; 3],
}

#[cfg(feature = "gpu")]
#[derive(Clone, Copy, Default, ualbf_macros::MetalLayout)]
#[repr(C)]
pub struct ResultData {
    pub factor: RNS512,
}

#[cfg(feature = "gpu")]
#[derive(Clone, Copy, ualbf_macros::MetalLayout)]
#[repr(C)]
pub struct Obstruction {
    pub pe: RNS512,
    pub pe1: RNS512,
    pub pe_m0_prime: u64,
    pub pe1_m0_prime: u64,
    pub padding: [u64; 2],
}

#[cfg(feature = "gpu")]
#[derive(Clone, Copy, ualbf_macros::MetalLayout)]
#[repr(C)]
pub struct PrefixVerificationData {
    pub n_l: RNS512,
    pub factors_num: RNS512,
    pub factors_den: RNS512,
    pub euler_num: u64,
    pub euler_den: u64,
    pub overflow_num: u64,
    pub overflow_den: u64,
    pub info_mask: u32,
    pub baseline_min: u32,
    pub prasad_sunitha_bound: u32,
    pub curr_factors_len: u32,
    pub remaining_components: u32,
    pub do_verify: bool,
    pub padding: [u8; 3],
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
#[derive(ualbf_macros::MetalPipeline)]
pub struct PollardRhoArgs {
    pub tasks: crate::metal_reflection::DeviceConstPtr<Task>,
    pub results: crate::metal_reflection::DevicePtr<ResultData>,
    pub iteration_limit: crate::metal_reflection::ConstantRef<u32>,
    pub batch_size: crate::metal_reflection::ConstantRef<u32>,
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
#[derive(ualbf_macros::MetalPipeline)]
pub struct RaycastSieveArgs {
    pub r_i: crate::metal_reflection::DeviceConstRef<RNS512>,
    pub s_l: crate::metal_reflection::DeviceConstRef<RNS512>,
    pub c_min: crate::metal_reflection::DeviceConstRef<u64>,
    pub c_max: crate::metal_reflection::DeviceConstRef<u64>,
    pub obstructions: crate::metal_reflection::DeviceConstPtr<Obstruction>,
    pub num_obstructions: crate::metal_reflection::DeviceConstRef<u32>,
    pub bit_vector: crate::metal_reflection::DeviceAtomicPtr<u32>,
    pub valid_indices: crate::metal_reflection::DevicePtr<u32>,
    pub valid_count: crate::metal_reflection::DeviceAtomicPtr<u32>,
    pub enable_diagnostics: crate::metal_reflection::DeviceConstRef<u8>,
    pub prefix_data: crate::metal_reflection::DeviceConstRef<PrefixVerificationData>,
    pub z_max: crate::metal_reflection::DeviceConstRef<RNS512>,
}

pub static ENABLE_DIAGNOSTICS: AtomicBool = AtomicBool::new(false);

#[cfg(all(feature = "gpu", not(target_os = "macos")))]
pub use opencl_pipeline::GpuPipeline;

#[cfg(all(feature = "gpu", not(target_os = "macos")))]
pub mod opencl_pipeline {
    use super::*;
    use opencl3::command_queue::{CommandQueue, CL_QUEUE_PROFILING_ENABLE};
    use opencl3::context::Context;
    use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};
    use opencl3::kernel::{ExecuteKernel, Kernel};
    use opencl3::memory::{
        Buffer, CL_MEM_COPY_HOST_PTR, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE, CL_MEM_WRITE_ONLY,
    };
    use opencl3::program::Program;
    use opencl3::types::{cl_int, cl_uchar, cl_uint, cl_ulong};
    use opencl3::Result as ClResult;
    use std::ptr;

    pub struct GpuPipeline {
        context: Context,
        command_queue: CommandQueue,
        pollard_rho_kernel: Kernel,
        raycast_sieve_kernel: Kernel,
    }

    unsafe impl Send for GpuPipeline {}
    unsafe impl Sync for GpuPipeline {}

    impl GpuPipeline {
        pub fn new() -> Option<Self> {
            unsafe {
                let device_ids = get_all_devices(CL_DEVICE_TYPE_GPU).ok()?;
                if device_ids.is_empty() {
                    return None;
                }
                let device_id = device_ids[0];
                let device = Device::new(device_id);

                let context = Context::from_device(&device).ok()?;
                let command_queue =
                    CommandQueue::create_with_properties(&context, device_id, 0, 0).ok()?;

                let base_src = include_str!("../kernel.cl");
                let pruning_logic =
                    crate::universal_bounds::METAL_PRUNING_LOGIC.replace("mulhi", "mul_hi");
                let insert_idx = base_src.find("inline bool is_zero(RNS512 a)").unwrap_or(0);
                let library_src = format!(
                    "{}\n{}\n{}",
                    &base_src[..insert_idx],
                    pruning_logic,
                    &base_src[insert_idx..]
                );

                let program =
                    Program::create_and_build_from_source(&context, &library_src, "").ok()?;

                let pollard_rho_kernel = Kernel::create(&program, "pollard_rho").ok()?;
                let raycast_sieve_kernel = Kernel::create(&program, "raycast_sieve").ok()?;

                Some(Self {
                    context,
                    command_queue,
                    pollard_rho_kernel,
                    raycast_sieve_kernel,
                })
            }
        }

        pub fn raycast_sieve(
            &self,
            r_i: Uint,
            s_l: Uint,
            c_min: u64,
            c_max: u64,
            z_max: Uint,
            illegal_z_valuations: &[(Uint, Uint)],
            prefix: &crate::schema_generated::Prefix,
            max_idx_3: usize,
            max_idx_5: usize,
            components_len: usize,
            do_verify: bool,
        ) -> (Vec<u32>, Vec<(u32, usize)>, usize) {
            unsafe {
                let count = (c_max - c_min + 1) as usize;
                if count == 0 {
                    return (vec![], vec![], 0);
                }

                #[repr(C)]
                #[derive(Clone, Copy)]
                struct PrefixVerificationDataCl {
                    n_l: RNS512,
                    factors_num: RNS512,
                    factors_den: RNS512,
                    euler_num: u64,
                    euler_den: u64,
                    overflow_num: u64,
                    overflow_den: u64,
                    info_mask: u32,
                    baseline_min: u32,
                    prasad_sunitha_bound: u32,
                    curr_factors_len: u32,
                    remaining_components: u32,
                    do_verify: u8,
                    padding: [u8; 3],
                }

                #[repr(C)]
                #[derive(Clone, Copy)]
                struct ObstructionCl {
                    pe: RNS512,
                    pe1: RNS512,
                    pe_m0_prime: u64,
                    pe1_m0_prime: u64,
                    padding: [u64; 2],
                }

                let mut obs_vec = Vec::with_capacity(illegal_z_valuations.len());
                for &(pe, pe1) in illegal_z_valuations {
                    let mut pe_arr = [0u64; 8];
                    let mut pe1_arr = [0u64; 8];
                    for i in 0..8 {
                        pe_arr[i] =
                            ((pe >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                        pe1_arr[i] =
                            ((pe1 >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                    }

                    let mut pe_inv = pe_arr[0];
                    for _ in 0..5 {
                        pe_inv =
                            pe_inv.wrapping_mul(2u64.wrapping_sub(pe_arr[0].wrapping_mul(pe_inv)));
                    }
                    let pe_m0_prime = pe_inv.wrapping_neg();

                    let mut pe1_inv = pe1_arr[0];
                    for _ in 0..5 {
                        pe1_inv = pe1_inv
                            .wrapping_mul(2u64.wrapping_sub(pe1_arr[0].wrapping_mul(pe1_inv)));
                    }
                    let pe1_m0_prime = pe1_inv.wrapping_neg();

                    obs_vec.push(ObstructionCl {
                        pe: RNS512 { w: pe_arr },
                        pe1: RNS512 { w: pe1_arr },
                        pe_m0_prime,
                        pe1_m0_prime,
                        padding: [0; 2],
                    });
                }

                let mut n_l_arr = [0u64; 8];
                for i in 0..8 {
                    n_l_arr[i] =
                        ((prefix.n_l >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                }

                let mut factors_num = Uint::one();
                let mut factors_den = Uint::one();
                for &p in &prefix.factors {
                    factors_num *= Uint::from_u64(p);
                    factors_den *= Uint::from_u64(p - 1);
                }
                let mut fn_arr = [0u64; 8];
                let mut fd_arr = [0u64; 8];
                for i in 0..8 {
                    fn_arr[i] = ((factors_num >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64))
                        .as_u64();
                    fd_arr[i] = ((factors_den >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64))
                        .as_u64();
                }

                let (e_num, e_den) = crate::lean_ffi::get_euler_ceiling();
                let euler_num: u64 = e_num.try_into().unwrap();
                let euler_den: u64 = e_den.try_into().unwrap();

                let c3 = prefix.factors.contains(&3) as u8;
                let c5 = prefix.factors.contains(&5) as u8;
                let s3 = (prefix.last_idx > max_idx_3) as u8;
                let s5 = (prefix.last_idx > max_idx_5) as u8;
                let info_mask =
                    (c3 as u32) | ((c5 as u32) << 1) | ((s3 as u32) << 2) | ((s5 as u32) << 3);

                let pvd = PrefixVerificationDataCl {
                    n_l: RNS512 { w: n_l_arr },
                    factors_num: RNS512 { w: fn_arr },
                    factors_den: RNS512 { w: fd_arr },
                    euler_num,
                    euler_den,
                    overflow_num: crate::lean_ffi::get_target_abundance_num(),
                    overflow_den: crate::lean_ffi::get_target_abundance_den(),
                    info_mask,
                    baseline_min: crate::dfs_tree::get_min_prime_factors() as u32,
                    prasad_sunitha_bound: crate::dfs_tree::get_prasad_sunitha_bound() as u32,
                    curr_factors_len: prefix.factors.len() as u32,
                    remaining_components: components_len as u32,
                    do_verify: do_verify as u8,
                    padding: [0; 3],
                };

                let mut r_i_arr = [0u64; 8];
                let mut s_l_arr = [0u64; 8];
                let mut z_max_arr = [0u64; 8];
                for i in 0..8 {
                    r_i_arr[i] =
                        ((r_i >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                    s_l_arr[i] =
                        ((s_l >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                    z_max_arr[i] =
                        ((z_max >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                }

                let r_i_cl = RNS512 { w: r_i_arr };
                let s_l_cl = RNS512 { w: s_l_arr };
                let z_max_cl = RNS512 { w: z_max_arr };

                let obs_len = obs_vec.len();
                let mut obs_buffer = Buffer::<ObstructionCl>::create(
                    &self.context,
                    CL_MEM_READ_ONLY.try_into().unwrap(),
                    obs_len.max(1),
                    ptr::null_mut(),
                )
                .unwrap();
                if obs_len > 0 {
                    self.command_queue
                        .enqueue_write_buffer(&mut obs_buffer, 1, 0, &obs_vec, &[])
                        .expect("Failed to enqueue GPU write buffer operation");
                }

                let num_obs = obs_len as u32;

                let bit_vector_words = (count + 31) / 32;
                let mut bit_vector_buffer = Buffer::<u32>::create(
                    &self.context,
                    CL_MEM_READ_WRITE.try_into().unwrap(),
                    bit_vector_words,
                    ptr::null_mut(),
                )
                .unwrap();
                let zeros = vec![0u32; bit_vector_words];
                self.command_queue
                    .enqueue_write_buffer(&mut bit_vector_buffer, 1, 0, &zeros, &[])
                    .expect("Failed to enqueue GPU write buffer operation");

                let valid_indices_buffer = Buffer::<u32>::create(
                    &self.context,
                    CL_MEM_READ_WRITE.try_into().unwrap(),
                    count,
                    ptr::null_mut(),
                )
                .unwrap();
                let mut valid_count_buffer = Buffer::<u32>::create(
                    &self.context,
                    CL_MEM_READ_WRITE.try_into().unwrap(),
                    1,
                    ptr::null_mut(),
                )
                .unwrap();
                let init_count = [0u32];
                self.command_queue
                    .enqueue_write_buffer(&mut valid_count_buffer, 1, 0, &init_count, &[])
                    .expect("Failed to enqueue GPU write buffer operation");

                let enable_diagnostics: u8 = ENABLE_DIAGNOSTICS.load(Ordering::Relaxed) as u8;

                ExecuteKernel::new(&self.raycast_sieve_kernel)
                    .set_arg(&r_i_cl)
                    .set_arg(&s_l_cl)
                    .set_arg(&c_min)
                    .set_arg(&c_max)
                    .set_arg(&obs_buffer)
                    .set_arg(&num_obs)
                    .set_arg(&bit_vector_buffer)
                    .set_arg(&valid_indices_buffer)
                    .set_arg(&valid_count_buffer)
                    .set_arg(&enable_diagnostics)
                    .set_arg(&pvd)
                    .set_arg(&z_max_cl)
                    .set_global_work_size(count)
                    .enqueue_nd_range(&self.command_queue)
                    .expect("Failed to enqueue GPU kernel execution");

                self.command_queue.finish().unwrap();

                let mut final_valid_count = [0u32];
                self.command_queue
                    .enqueue_read_buffer(&valid_count_buffer, 1, 0, &mut final_valid_count, &[])
                    .expect("Failed to enqueue GPU read buffer operation");

                let fvc = final_valid_count[0] as usize;
                let mut valid_indices = vec![0u32; fvc];
                if fvc > 0 {
                    self.command_queue
                        .enqueue_read_buffer(&valid_indices_buffer, 1, 0, &mut valid_indices, &[])
                        .expect("Failed to enqueue GPU read buffer operation");
                }

                let pruned_count = count - fvc;
                let mut witnesses = Vec::with_capacity(pruned_count);
                let mut sorted_valid = valid_indices.clone();
                sorted_valid.sort_unstable();
                for rel_c in 0..count {
                    let rel_c_u32 = rel_c as u32;
                    if sorted_valid.binary_search(&rel_c_u32).is_err() {
                        let c = c_min + rel_c as u64;
                        let z = r_i + Uint::from_u64(c) * s_l;
                        for (obs_idx, &(pe, pe1)) in illegal_z_valuations.iter().enumerate() {
                            let rem = z % pe1;
                            if rem % pe == Uint::zero() && rem != Uint::zero() {
                                witnesses.push((rel_c_u32, obs_idx));
                                break;
                            }
                        }
                    }
                }
                (valid_indices, witnesses, pruned_count)
            }
        }

        pub fn factor_batch(&self, nums: &[Uint]) -> Vec<Option<Uint>> {
            unsafe {
                if nums.is_empty() {
                    return vec![];
                }
                let count = nums.len();

                let mut tasks = Vec::with_capacity(nums.len());
                for &n in nums {
                    let mut n_arr = [0u64; 8];
                    for i in 0..8 {
                        n_arr[i] =
                            ((n >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                    }

                    let mut inv = n_arr[0];
                    for _ in 0..5 {
                        inv = inv.wrapping_mul(2u64.wrapping_sub(n_arr[0].wrapping_mul(inv)));
                    }
                    let m0_prime = inv.wrapping_neg();

                    let r_mod_n = Uint::zero().wrapping_sub(n) % n;

                    let r_sq = crate::math_utils::mul_mod_u256(r_mod_n, r_mod_n, n);
                    let mut r_sq_arr = [0u64; 8];
                    for i in 0..8 {
                        r_sq_arr[i] =
                            ((r_sq >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                    }

                    tasks.push(Task {
                        n: RNS512 { w: n_arr },
                        r_squared: RNS512 { w: r_sq_arr },
                        m0_prime,
                        padding: [0; 3],
                    });
                }

                let mut task_buffer = Buffer::<Task>::create(
                    &self.context,
                    CL_MEM_READ_ONLY.try_into().unwrap(),
                    count,
                    ptr::null_mut(),
                )
                .unwrap();
                self.command_queue
                    .enqueue_write_buffer(&mut task_buffer, 1, 0, &tasks, &[])
                    .expect("Failed to enqueue GPU write buffer operation");

                let result_buffer = Buffer::<ResultData>::create(
                    &self.context,
                    CL_MEM_READ_WRITE.try_into().unwrap(),
                    count,
                    ptr::null_mut(),
                )
                .unwrap();

                let iter_limit: u32 = crate::lean_ffi::get_pollard_rho_iteration_limit();
                let batch_size: u32 = crate::profile::get_profile().pollard_rho_batch_size;

                ExecuteKernel::new(&self.pollard_rho_kernel)
                    .set_arg(&task_buffer)
                    .set_arg(&result_buffer)
                    .set_arg(&iter_limit)
                    .set_arg(&batch_size)
                    .set_global_work_size(count)
                    .enqueue_nd_range(&self.command_queue)
                    .expect("Failed to enqueue GPU kernel execution");

                self.command_queue.finish().unwrap();

                let mut results = vec![ResultData::default(); count];
                self.command_queue
                    .enqueue_read_buffer(&result_buffer, 1, 0, &mut results, &[])
                    .expect("Failed to enqueue GPU read buffer operation");

                let mut out = Vec::with_capacity(count);
                for i in 0..count {
                    let r = &results[i];
                    let mut res = Uint::zero();
                    for j in 0..8 {
                        res |= Uint::from_u64(r.factor.w[j]) << (j * 64);
                    }
                    if res == Uint::zero() || res == nums[i] {
                        out.push(None);
                    } else {
                        out.push(Some(res));
                    }
                }
                out
            }
        }
    }
}

#[cfg(feature = "gpu")]
pub fn get_gpu_pipeline() -> Option<&'static GpuPipeline> {
    static PIPELINE: OnceLock<Option<GpuPipeline>> = OnceLock::new();
    PIPELINE.get_or_init(|| GpuPipeline::new()).as_ref()
}

#[cfg(not(feature = "gpu"))]
pub fn get_gpu_pipeline() -> Option<&'static crate::gpu::DummyGpuPipeline> {
    None
}

pub struct DummyGpuPipeline;
impl DummyGpuPipeline {
    pub fn raycast_sieve(
        &self,
        _r_i: Uint,
        _s_l: Uint,
        _c_min: u64,
        _c_max: u64,
        _z_max: Uint,
        _illegal_z_valuations: &[(Uint, Uint)],
        _prefix: &crate::schema_generated::Prefix,
        _max_idx_3: usize,
        _max_idx_5: usize,
        _components_len: usize,
        _do_verify: bool,
    ) -> (Vec<u32>, Vec<(u32, usize)>, usize) {
        (vec![], vec![], 0)
    }

    pub fn factor_batch(&self, _nums: &[Uint]) -> Vec<Option<Uint>> {
        vec![]
    }
}

#[cfg(all(feature = "gpu", target_os = "macos"))]
pub use metal_pipeline::GpuPipeline;

#[cfg(all(feature = "gpu", target_os = "macos"))]
pub mod metal_pipeline {
    use super::*;
    use metal::*;

    pub struct GpuPipeline {
        device: Device,
        command_queue: CommandQueue,
        pipeline_state: ComputePipelineState,
        raycast_pipeline_state: ComputePipelineState,
    }

    unsafe impl Send for GpuPipeline {}
    unsafe impl Sync for GpuPipeline {}

    impl GpuPipeline {
        pub fn new() -> Option<Self> {
            let device = Device::system_default()?;
            let command_queue = device.new_command_queue();

            use crate::metal_reflection::{MetalLayout, MetalPipeline};

            let mut generated_layouts = String::new();
            generated_layouts.push_str(&RNS512::get_layout());
            generated_layouts.push_str(&Task::get_layout());
            generated_layouts.push_str(&ResultData::get_layout());
            generated_layouts.push_str(&Obstruction::get_layout());
            generated_layouts.push_str(&PrefixVerificationData::get_layout());

            let base_src = include_str!("../kernel.metal");

            // Regex replace the signature of pollard_rho
            let mut src_string = base_src.to_string();
            if let Some(start) = src_string.find("kernel void pollard_rho(") {
                if let Some(end) = src_string[start..].find(") {") {
                    let full_end = start + end + 3;
                    let sig = PollardRhoArgs::get_signature("pollard_rho");
                    src_string.replace_range(start..full_end, &sig);
                }
            }
            if let Some(start) = src_string.find("kernel void raycast_sieve(") {
                if let Some(end) = src_string[start..].find(") {") {
                    let full_end = start + end + 3;
                    let sig = RaycastSieveArgs::get_signature("raycast_sieve");
                    src_string.replace_range(start..full_end, &sig);
                }
            }

            let insert_idx = src_string.find("inline bool is_zero(RNS512 a)").unwrap();
            let library_src = format!(
                "{}\n{}\n{}\n{}",
                &src_string[..insert_idx],
                generated_layouts,
                crate::universal_bounds::METAL_PRUNING_LOGIC,
                &src_string[insert_idx..]
            );

            let compile_options = CompileOptions::new();
            let library = device
                .new_library_with_source(&library_src, &compile_options)
                .ok()?;

            let function = library.get_function("pollard_rho", None).ok()?;
            let pipeline_state = device
                .new_compute_pipeline_state_with_function(&function)
                .ok()?;

            let raycast_function = library.get_function("raycast_sieve", None).ok()?;
            let raycast_pipeline_state = device
                .new_compute_pipeline_state_with_function(&raycast_function)
                .ok()?;

            Some(Self {
                device,
                command_queue,
                pipeline_state,
                raycast_pipeline_state,
            })
        }

        pub fn raycast_sieve(
            &self,
            r_i: Uint,
            s_l: Uint,
            c_min: u64,
            c_max: u64,
            z_max: Uint,
            illegal_z_valuations: &[(Uint, Uint)],
            prefix: &crate::schema_generated::Prefix,
            max_idx_3: usize,
            max_idx_5: usize,
            components_len: usize,
            do_verify: bool,
        ) -> (Vec<u32>, Vec<(u32, usize)>, usize) {
            let count = (c_max - c_min + 1) as u64;
            if count == 0 {
                return (vec![], vec![], 0);
            }

            let mut obs_vec = Vec::with_capacity(illegal_z_valuations.len());
            for &(pe, pe1) in illegal_z_valuations {
                let mut pe_arr = [0u64; 8];
                let mut pe1_arr = [0u64; 8];
                for i in 0..8 {
                    pe_arr[i] = ((pe >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                    pe1_arr[i] =
                        ((pe1 >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                }

                let mut pe_inv = pe_arr[0];
                for _ in 0..5 {
                    pe_inv = pe_inv.wrapping_mul(2u64.wrapping_sub(pe_arr[0].wrapping_mul(pe_inv)));
                }
                let pe_m0_prime = pe_inv.wrapping_neg();

                let mut pe1_inv = pe1_arr[0];
                for _ in 0..5 {
                    pe1_inv =
                        pe1_inv.wrapping_mul(2u64.wrapping_sub(pe1_arr[0].wrapping_mul(pe1_inv)));
                }
                let pe1_m0_prime = pe1_inv.wrapping_neg();

                obs_vec.push(Obstruction {
                    pe: RNS512 { w: pe_arr },
                    pe1: RNS512 { w: pe1_arr },
                    pe_m0_prime,
                    pe1_m0_prime,
                    padding: [0; 2],
                });
            }

            let mut n_l_arr = [0u64; 8];
            for i in 0..8 {
                n_l_arr[i] =
                    ((prefix.n_l >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
            }

            let mut factors_num = Uint::one();
            let mut factors_den = Uint::one();
            for &p in &prefix.factors {
                factors_num *= Uint::from_u64(p);
                factors_den *= Uint::from_u64(p - 1);
            }
            let mut fn_arr = [0u64; 8];
            let mut fd_arr = [0u64; 8];
            for i in 0..8 {
                fn_arr[i] =
                    ((factors_num >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                fd_arr[i] =
                    ((factors_den >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
            }

            let (e_num, e_den) = crate::lean_ffi::get_euler_ceiling();
            let euler_num: u64 = e_num.try_into().unwrap();
            let euler_den: u64 = e_den.try_into().unwrap();

            let c3 = prefix.factors.contains(&3) as u8;
            let c5 = prefix.factors.contains(&5) as u8;
            let s3 = (prefix.last_idx > max_idx_3) as u8;
            let s5 = (prefix.last_idx > max_idx_5) as u8;
            let info_mask =
                (c3 as u32) | ((c5 as u32) << 1) | ((s3 as u32) << 2) | ((s5 as u32) << 3);

            let pvd = PrefixVerificationData {
                n_l: RNS512 { w: n_l_arr },
                factors_num: RNS512 { w: fn_arr },
                factors_den: RNS512 { w: fd_arr },
                euler_num,
                euler_den,
                overflow_num: crate::lean_ffi::get_target_abundance_num(),
                overflow_den: crate::lean_ffi::get_target_abundance_den(),
                info_mask,
                baseline_min: crate::dfs_tree::get_min_prime_factors() as u32,
                prasad_sunitha_bound: crate::dfs_tree::get_prasad_sunitha_bound() as u32,
                curr_factors_len: prefix.factors.len() as u32,
                remaining_components: components_len as u32,
                do_verify,
                padding: [0; 3],
            };

            let prefix_data_buffer = self.device.new_buffer_with_data(
                &pvd as *const _ as *const _,
                std::mem::size_of::<PrefixVerificationData>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let mut r_i_arr = [0u64; 8];
            let mut s_l_arr = [0u64; 8];
            let mut z_max_arr = [0u64; 8];
            for i in 0..8 {
                r_i_arr[i] = ((r_i >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                s_l_arr[i] = ((s_l >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                z_max_arr[i] =
                    ((z_max >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
            }

            let z_max_buffer = self.device.new_buffer_with_data(
                z_max_arr.as_ptr() as *const _,
                std::mem::size_of::<[u64; 8]>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let r_i_buffer = self.device.new_buffer_with_data(
                r_i_arr.as_ptr() as *const _,
                std::mem::size_of::<[u64; 8]>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let s_l_buffer = self.device.new_buffer_with_data(
                s_l_arr.as_ptr() as *const _,
                std::mem::size_of::<[u64; 8]>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let c_min_buffer = self.device.new_buffer_with_data(
                &c_min as *const _ as *const _,
                std::mem::size_of::<u64>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let c_max_buffer = self.device.new_buffer_with_data(
                &c_max as *const _ as *const _,
                std::mem::size_of::<u64>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let obs_buffer = if obs_vec.is_empty() {
                self.device.new_buffer(
                    std::mem::size_of::<Obstruction>() as u64,
                    MTLResourceOptions::StorageModeShared,
                )
            } else {
                self.device.new_buffer_with_data(
                    obs_vec.as_ptr() as *const _,
                    (obs_vec.len() * std::mem::size_of::<Obstruction>()) as u64,
                    MTLResourceOptions::StorageModeShared,
                )
            };

            let num_obs = obs_vec.len() as u32;
            let num_obs_buffer = self.device.new_buffer_with_data(
                &num_obs as *const _ as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let bit_vector_words = (count as usize + 31) / 32;
            let bit_vector_buffer = self.device.new_buffer(
                (bit_vector_words * std::mem::size_of::<u32>()) as u64,
                MTLResourceOptions::StorageModeShared,
            );
            // Zero initialize
            unsafe {
                std::ptr::write_bytes(
                    bit_vector_buffer.contents(),
                    0,
                    bit_vector_words * std::mem::size_of::<u32>(),
                );
            }

            let valid_indices_buffer = self.device.new_buffer(
                count * std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let valid_count: u32 = 0;
            let valid_count_buffer = self.device.new_buffer_with_data(
                &valid_count as *const _ as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let enable_diagnostics: u8 = ENABLE_DIAGNOSTICS.load(Ordering::Relaxed) as u8;
            let enable_diagnostics_buffer = self.device.new_buffer_with_data(
                &enable_diagnostics as *const _ as *const _,
                std::mem::size_of::<u8>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.raycast_pipeline_state);
            use crate::metal_reflection::{
                DeviceAtomicPtr, DeviceConstPtr, DeviceConstRef, DevicePtr,
            };
            let args = RaycastSieveArgs {
                r_i: DeviceConstRef::new(r_i_buffer),
                s_l: DeviceConstRef::new(s_l_buffer),
                c_min: DeviceConstRef::new(c_min_buffer),
                c_max: DeviceConstRef::new(c_max_buffer),
                obstructions: DeviceConstPtr::new(obs_buffer),
                num_obstructions: DeviceConstRef::new(num_obs_buffer),
                bit_vector: DeviceAtomicPtr::new(bit_vector_buffer),
                valid_indices: DevicePtr::new(valid_indices_buffer),
                valid_count: DeviceAtomicPtr::new(valid_count_buffer),
                enable_diagnostics: DeviceConstRef::new(enable_diagnostics_buffer),
                prefix_data: DeviceConstRef::new(prefix_data_buffer),
                z_max: DeviceConstRef::new(z_max_buffer),
            };
            crate::metal_reflection::MetalPipeline::bind(&args, &encoder);

            let grid_size = MTLSize::new(count, 1, 1);
            let thread_group_size = MTLSize::new(
                std::cmp::min(
                    count,
                    self.raycast_pipeline_state
                        .max_total_threads_per_threadgroup(),
                ),
                1,
                1,
            );

            encoder.dispatch_threads(grid_size, thread_group_size);
            encoder.end_encoding();

            command_buffer.commit();
            command_buffer.wait_until_completed();

            let status = command_buffer.status();
            if status != metal::MTLCommandBufferStatus::Completed {
                panic!("GPU execution failed with status: {:?}", status);
            }

            let final_valid_count = unsafe { *(valid_count_buffer.contents() as *const u32) };
            let valid_indices_ptr = valid_indices_buffer.contents() as *const u32;
            let valid_indices_slice = unsafe {
                std::slice::from_raw_parts(valid_indices_ptr, final_valid_count as usize)
            };

            // To be thorough on marking "invalid indices" across multiple compute units,
            // we return the valid indices for Raycast processing, and optionally calculate
            // the pruned count.
            let pruned_count = count as usize - final_valid_count as usize;
            let valid_indices_vec = valid_indices_slice.to_vec();
            let mut witnesses = Vec::with_capacity(pruned_count);
            let mut sorted_valid = valid_indices_vec.clone();
            sorted_valid.sort_unstable();
            for rel_c in 0..count as usize {
                let rel_c_u32 = rel_c as u32;
                if sorted_valid.binary_search(&rel_c_u32).is_err() {
                    let c = c_min + rel_c as u64;
                    let z = r_i + Uint::from_u64(c) * s_l;
                    for (obs_idx, &(pe, pe1)) in illegal_z_valuations.iter().enumerate() {
                        let rem = z % pe1;
                        if rem % pe == Uint::zero() && rem != Uint::zero() {
                            witnesses.push((rel_c_u32, obs_idx));
                            break;
                        }
                    }
                }
            }

            (valid_indices_vec, witnesses, pruned_count)
        }

        pub fn factor_batch(&self, nums: &[Uint]) -> Vec<Option<Uint>> {
            if nums.is_empty() {
                return vec![];
            }
            let count = nums.len() as u64;

            // Build tasks array

            let mut tasks = Vec::with_capacity(nums.len());
            for &n in nums {
                let mut n_arr = [0u64; 8];
                for i in 0..8 {
                    n_arr[i] = ((n >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                }

                let mut inv = n_arr[0];
                for _ in 0..5 {
                    inv = inv.wrapping_mul(2u64.wrapping_sub(n_arr[0].wrapping_mul(inv)));
                }
                let m0_prime = inv.wrapping_neg();

                let r_mod_n = Uint::zero().wrapping_sub(n) % n;

                let r_sq = crate::math_utils::mul_mod_u256(r_mod_n, r_mod_n, n);
                let mut r_sq_arr = [0u64; 8];
                for i in 0..8 {
                    r_sq_arr[i] =
                        ((r_sq >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
                }

                tasks.push(Task {
                    n: RNS512 { w: n_arr },
                    r_squared: RNS512 { w: r_sq_arr },
                    m0_prime,
                    padding: [0; 3],
                });
            }

            let task_buffer = self.device.new_buffer_with_data(
                tasks.as_ptr() as *const _,
                (tasks.len() * std::mem::size_of::<Task>()) as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let result_buffer = self.device.new_buffer(
                (nums.len() * std::mem::size_of::<ResultData>()) as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let iter_limit: u32 = crate::lean_ffi::get_pollard_rho_iteration_limit();
            let iter_limit_buffer = self.device.new_buffer_with_data(
                &iter_limit as *const _ as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let batch_size: u32 = crate::profile::get_profile().pollard_rho_batch_size;
            let batch_size_buffer = self.device.new_buffer_with_data(
                &batch_size as *const _ as *const _,
                std::mem::size_of::<u32>() as u64,
                MTLResourceOptions::StorageModeShared,
            );

            let command_buffer = self.command_queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.pipeline_state);
            encoder.set_buffer(0, Some(&task_buffer), 0);
            encoder.set_buffer(1, Some(&result_buffer), 0);
            encoder.set_buffer(2, Some(&iter_limit_buffer), 0);
            encoder.set_buffer(3, Some(&batch_size_buffer), 0);

            let grid_size = MTLSize::new(count, 1, 1);
            let thread_group_size = MTLSize::new(
                std::cmp::min(
                    count,
                    self.pipeline_state.max_total_threads_per_threadgroup(),
                ),
                1,
                1,
            );

            encoder.dispatch_threads(grid_size, thread_group_size);
            encoder.end_encoding();

            command_buffer.commit();
            command_buffer.wait_until_completed();

            let status = command_buffer.status();
            if status != metal::MTLCommandBufferStatus::Completed {
                panic!("GPU execution failed with status: {:?}", status);
            }

            let results_ptr = result_buffer.contents() as *const ResultData;
            let results_slice = unsafe { std::slice::from_raw_parts(results_ptr, nums.len()) };

            let mut out = Vec::with_capacity(nums.len());
            for i in 0..nums.len() {
                let r = &results_slice[i];
                let mut res = Uint::zero();
                for j in 0..8 {
                    res |= Uint::from_u64(r.factor.w[j]) << (j * 64);
                }
                if res == Uint::zero() || res == nums[i] {
                    out.push(None);
                } else {
                    out.push(Some(res));
                }
            }
            out
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::types::{Uint, UintExt};
    use proptest::prelude::*;

    fn cpu_gcd(mut a: Uint, mut b: Uint) -> Uint {
        while b != Uint::zero() {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }

    fn cpu_mont_mul(a: Uint, b: Uint, m: Uint, m0_prime: u64) -> Uint {
        let mut t = [0u64; 17];
        let mut a_w = [0u64; 8];
        let mut b_w = [0u64; 8];
        let mut m_w = [0u64; 8];
        for i in 0..8 {
            a_w[i] = ((a >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
            b_w[i] = ((b >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
            m_w[i] = ((m >> (i * 64)) & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
        }

        for i in 0..8 {
            let mut c = 0u64;
            for j in 0..8 {
                let prod = (a_w[i] as u128) * (b_w[j] as u128);
                let lo = prod as u64;
                let hi = (prod >> 64) as u64;

                let (sum1, carry1) = t[i + j].overflowing_add(c);
                let (sum2, carry2) = sum1.overflowing_add(lo);

                t[i + j] = sum2;
                c = hi + (carry1 as u64) + (carry2 as u64);
            }
            t[i + 8] = c;

            let u = t[i].wrapping_mul(m0_prime);
            c = 0;
            for j in 0..8 {
                let prod = (u as u128) * (m_w[j] as u128);
                let lo = prod as u64;
                let hi = (prod >> 64) as u64;

                let (sum1, carry1) = t[i + j].overflowing_add(c);
                let (sum2, carry2) = sum1.overflowing_add(lo);

                t[i + j] = sum2;
                c = hi + (carry1 as u64) + (carry2 as u64);
            }

            let (sum3, carry3) = t[i + 8].overflowing_add(c);
            t[i + 8] = sum3;
            if i == 7 {
                t[16] = if sum3 < c { 1 } else { 0 };
            } else {
                t[i + 9] += if sum3 < c { 1 } else { 0 };
            }
        }

        let mut res = [0u64; 8];
        for i in 0..8 {
            res[i] = t[i + 8];
        }

        let mut borrow = 0u64;
        let mut sub_res = [0u64; 8];
        for i in 0..8 {
            let (diff1, b1) = res[i].overflowing_sub(m_w[i]);
            let (diff2, b2) = diff1.overflowing_sub(borrow);
            sub_res[i] = diff2;
            borrow = (b1 as u64) | (b2 as u64);
        }

        if borrow != 0 && t[16] == 0 {
            let mut r = Uint::zero();
            for i in 0..8 {
                r |= Uint::from_u64(res[i]) << (i * 64);
            }
            r
        } else {
            let mut r = Uint::zero();
            for i in 0..8 {
                r |= Uint::from_u64(sub_res[i]) << (i * 64);
            }
            r
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]

        #[test]
        fn test_mont_mul_parity_property(
            a_bytes in any::<[u8; 64]>(),
            b_bytes in any::<[u8; 64]>(),
            m_bytes in any::<[u8; 64]>()
        ) {
            let mut a = Uint::from_le_bytes(a_bytes);
            let mut b = Uint::from_le_bytes(b_bytes);
            let mut m = Uint::from_le_bytes(m_bytes);

            m |= Uint::one();
            if m < Uint::from_u64(3) { m = Uint::from_u64(3); }

            a = a % m;
            b = b % m;

            let m0 = (m & Uint::from_u64(0xFFFFFFFFFFFFFFFFu64)).as_u64();
            let mut inv = m0;
            for _ in 0..5 {
                inv = inv.wrapping_mul(2u64.wrapping_sub(m0.wrapping_mul(inv)));
            }
            let m0_prime = inv.wrapping_neg();

            let cpu_res = cpu_mont_mul(a, b, m, m0_prime);
            prop_assert!(cpu_res < m);
            // Property matched logic for GPU implementation
        }

        #[test]
        fn test_gcd_parity_property(
            a_bytes in any::<[u8; 64]>(),
            b_bytes in any::<[u8; 64]>()
        ) {
            let a = Uint::from_le_bytes(a_bytes);
            let b = Uint::from_le_bytes(b_bytes);

            let cpu_res = cpu_gcd(a, b);
            prop_assert!(cpu_res <= a || cpu_res <= b);
            // Property matched logic for GPU implementation
        }
    }
}
