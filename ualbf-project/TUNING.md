# Structured Performance Profiles — Tuning Guide

The UALBF Engine allows researchers to modify search parameters dynamically using a `rust-engine/profile.json.template` file. This prevents the need for manual code recompilation and enables easy hardware benchmarking without changing the mathematically verified bounds.

## Profile Configuration Format

The engine expects a `rust-engine/profile.json.template` file in the execution directory with the following structure:

```json
{
  "pollard_rho_batch_size": 128,
  "active_prime_slots": 64,
  "engine_telemetry_interval_ms": 1000,
  "dashboard_telemetry_interval_ms": 250
}
```

### Parameter Details

- **POLLARD_RHO_BATCH_SIZE**
  - **Description**: The number of polynomial iterations ($f(x) = x^2 + 1$) performed before applying the GCD step in the Pollard-rho fallback factorization logic.
  - **Tuning Strategy**: For modern processors with deep instruction pipelines, larger batch sizes (e.g., 256 or 512) increase throughput by amortizing the cost of the GCD check. For very large factors or on low-cache systems, smaller sizes (e.g., 64) might prove more responsive.

- **active_prime_slots**
  - **Description**: Determines the number of lock-free telemetry slots available for worker threads to report their currently processing roots.
  - **Tuning Strategy**: Set this value to be greater than or equal to the number of physical CPU cores on the execution machine. For a 128-core system, a value of `128` or `256` guarantees thread visibility without telemetry contention.

- **engine_telemetry_interval_ms**
  - **Description**: The minimum elapsed time (in milliseconds) before the core search loops (`sieve` and `dfs_tree`) output progress trace logs.
  - **Tuning Strategy**: Lowering this value (e.g., `100` ms) provides higher fidelity logs at a slight cost to I/O processing bandwidth. For high-throughput production runs, this value can be safely raised to `5000` or `10000` to silence logs and minimize CPU interrupts.

- **dashboard_telemetry_interval_ms**
  - **Description**: The update frequency for the curses GUI dashboard interface.
  - **Tuning Strategy**: Decreasing the interval leads to smoother UI frame rates (e.g., `50` ms yields 20 FPS). However, updating too frequently steals rendering cycles from the main Python thread. `250` ms is recommended for standard monitoring.

### Mathematical Bounds and Parameter Configurations

In addition to user-tunable performance configurations, the engine depends on verified mathematical limits defined in `bounds_manifest.json`:

- **omega_bounds.prasad_sunitha.proof_bound**
  - **Description**: The mathematical lower bound on the number of prime factors of quasiperfect numbers proven by Prasad & Sunitha.

- **omega_bounds.prasad_sunitha.engine_justified_gap**
  - **Description**: The gap between the Prasad-Sunitha proved bound and the engine's capability.

- **omega_bounds.div_5_coprime_3.proof_bound**
  - **Description**: The mathematical lower bound on the number of prime factors of quasiperfect numbers divisible by 5 but coprime to 3 (Hagis & Cohen 1982).

- **omega_bounds.div_5_coprime_3.engine_justified_gap**
  - **Description**: The gap between the div_5_coprime_3 proved bound and the engine's capability.

- **omega_bounds.hagis1982.proof_bound**
  - **Description**: The baseline minimum number of prime factors proven by Hagis (1982).

- **omega_bounds.hagis1982.engine_justified_gap**
  - **Description**: The gap between the Hagis baseline bound and the engine.

- **search_bounds.target_max_log10.value**
  - **Description**: Upper log10 bound constraint for candidate searches.

- **search_bounds.sieve_limit.value**
  - **Description**: Sieve limit size used to find small prime factors during preprocessing.

- **search_bounds.max_exponent.value**
  - **Description**: Maximum allowable prime exponent checked during factor search.

- **search_bounds.prefix_stop_threshold.value**
  - **Description**: The numeric value threshold at which the prefix search stops checking deeper branches.

- **search_bounds.target_min_log10.value**
  - **Description**: Lower log10 bound constraint for candidate searches.

- **search_bounds.pollard_rho.iteration_limit**
  - **Description**: Maximum number of iteration steps performed by the Pollard-rho fallback algorithm.

- **search_bounds.pollard_rho.batch_size**
  - **Description**: Batch size for checking GCDs during Pollard-rho factorization.

- **search_bounds.raycast.gpu_threshold**
  - **Description**: Threshold for offloading raycast calculations to GPU buffers.

- **search_bounds.raycast.chunk_size**
  - **Description**: Chunk processing size for GPU offloading of raycasts.

- **search_bounds.prime_split_threshold.value**
  - **Description**: The prime split threshold parameter used to decouple mathematical proofs of infinite tail products from hardcoded static prime threshold values.

- **euler_ceiling.num**
  - **Description**: Numerator of the Euler ceiling quotient bound.

- **euler_ceiling.den**
  - **Description**: Denominator of the Euler ceiling quotient bound.

- **overflow_threshold.num**
  - **Description**: Numerator of the overflow safety threshold ratio.

- **overflow_threshold.den**
  - **Description**: Denominator of the overflow safety threshold ratio.

- **prime_factor_list**
  - **Description**: Array of pre-loaded small prime numbers used during sieving operations.

- **static_suffix_bounds**
  - **Description**: Array of static limits mapping factors to their pre-calculated mathematical suffixes.

- **conjectural_bounds.active**
  - **Description**: Flag to enable or disable the use of conjectural bounds (e.g. from the ABC Conjecture) during target search.

- **conjectural_bounds.target_max_log10_ceiling**
  - **Description**: The maximum upper log10 bound ceiling applied when conjectural bounds are active.

- **touchard_mod_24.modulus**
  - **Description**: The modulus used for checking Touchard congruence.

- **touchard_mod_24.residues**
  - **Description**: The valid mod-24 residue set for quasiperfect numbers.

- **crt_obstruction.moduli**
  - **Description**: The list of moduli used during Chinese Remainder Theorem (CRT) obstruction checks.

- **crt_obstruction.modulus_product**
  - **Description**: The product of the CRT moduli, defining the period of the obstruction pattern.

- **lattice_precision_tolerance**
  - **Description**: The float value specifying precision tolerance for lattice certifications.

- **lattice_precision_tolerance**
  - **Description**: Precision tolerance threshold for floating point and lattice-based computations in the engine.

- **lattice_precision_tolerance**
  - **Description**: Floating-point precision tolerance used during lattice reduction and bounds calculations.

## Fallback Defaults
If the file is absent or improperly formatted, the engine applies hardcoded safe defaults:
- POLLARD_RHO_BATCH_SIZE: 128
- active_prime_slots: 64
- engine_telemetry_interval_ms: 1000
- dashboard_telemetry_interval_ms: 250
