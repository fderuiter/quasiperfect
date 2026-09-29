# Trusted Computing Base (TCB) & Verification Boundaries

This document defines the Trusted Computing Base (TCB) for the Unified Algebraic-Lattice Bipartition Framework (UALBF). To maintain transparency and strict mathematical certitude, we explicitly disclose the boundaries of our formally verified claims. The components listed below act as unverified external blocks, FFI boundaries, or trusted mathematical assumptions rather than mechanically checked proofs.

## 1. Lean-to-Rust FFI Boundary
The Foreign Function Interface (FFI) bridging the Lean 4 formalization and the Rust execution engine is unverified.
- **Current State:** The Rust execution engine relies on C-compatible data serialization and exported semantics via Lean's `@[export]` pragmas.
- **Verification Status:** While the individual Lean 4 proofs are mechanically checked and the Rust execution logic is highly robust, the bridging logic across the boundary itself forms a critical part of the TCB and is not formally proven.

## 2. Bloom Filter Hashing Primitives
The Bloom filter's wrapping double-hashing logic is formally verified in Lean 4 to have zero false negatives. However, the underlying cryptographic (SHA-256) and multiplicative (FNV-1a) hash primitives that generate the initial hash seeds are excluded from formal verification.
- **Current State:** The Lean 4 formalization guarantees that the index generation step maps inputs securely to the bitset, but relies on Rust-side unverified implementations of SHA-256 and FNV-1a.
- **Verification Status:** The hash primitives themselves form part of the TCB and remain unverified.

## 3. Miller-Rabin Verification Boundaries
The verification pipeline does not treat the 20-base Miller-Rabin sufficiency test as an active, trusted mathematical axiom. To preserve performance guarantees while maintaining complete mathematical certitude and transparency, the system configuration manifest and verification layer reject the assumption that any 20-base probabilistic check is axiomatically sufficient for primality. This is formally represented by the spec function `lean_miller_rabin_20_base_sufficiency` and the system bounds manifest `bounds_manifest.json`, where the `is_axiomatic` status is set to `false`.

Instead of probabilistic sufficiency assumptions, the framework employs a hybrid tiered primality pipeline in the `verified_is_prime` function:

- **Inputs Below 2^64 (Smaller Candidate Primes):**
  All candidate pruning decisions for numbers under 2^64 emit explicit, machine-checkable compositeness witness certificates (verifiable divisor factors or Miller-Rabin witness bases). The proof validation pipeline ingests and mathematically verifies each compositeness witness certificate, refuting primality without relying on an unverified 12-base screening assumption in the Trusted Computing Base.
  
- **Inputs Equal to or Exceeding 2^64 (Larger Candidate Primes):**
  Inputs at or above this boundary cannot be verified solely using probabilistic Miller-Rabin checks. Instead, they are subjected to a rigorous certificate-backed verification pathway. The 20-base Miller-Rabin check is used strictly as a fast, non-binding pre-filter to reject composite candidates. Any candidate that passes this pre-filter must be validated using a mathematically rigorous, verified Pocklington certificate via `generate_and_verify_pocklington` for absolute certitude. This certificate-backed pathway is the mandatory mechanism for all inputs equal to or exceeding 2^64.

## 4. GPU Sieve Backends and Witness Verification Gateway
The CRT tensor sieve has optional Metal and OpenCL backends in `rust-engine/src/unverified/gpu.rs`. They sit outside the formally verified TCB.
- **Current State:** Both hardware backends are compiled only behind a `gpu` Cargo feature, which `rust-engine/Cargo.toml` does not currently declare. Standard builds therefore use `DummyGpuPipeline`, which returns an empty bitmap, and every sieve decision is made by the CPU (Rayon) path. No GPU code runs in CI or in published certificates.
- **Verification Status:** If a hardware backend is enabled (it also requires `UALBF_ALLOW_UNVERIFIED_GPU`), each eliminated component is emitted as a `GpuBloomWitness`. A host-side gateway re-checks every witness against `ualbf_check_crt_1155_sound` / `check_crt_1155` from `UALBF/Engine/Mod1155Bridge.lean` before any candidate is pruned, and halts GPU execution on the first failure.

## 4a. Trusted Mathematical Axioms
One literature result is assumed rather than proved in Lean, and it is the only entry in `ALLOWED_AXIOMS` (`cert_util.py`):
- `UALBF.QPN.PrasadSunitha.qpn_div_5_coprime_3_omega_bound` (`UALBF/QPN/PrasadSunitha.lean`): a quasiperfect number divisible by 5 and coprime to 3 has at least 11 distinct prime factors (Hagis and Cohen, 1982). It is marked `is_axiomatic: true` in `bounds_manifest.json`, and the auditor rejects any other axiom.

## 5. Build Environment Variables & Verification Configuration

All build tools, certificate verification scripts, and paper generation utilities strictly validate environment variables against `env_manifest.json` and `env_manifest.schema.json`.

| Variable | Type | Default | Status | Description |
|---|---|---|---|---|
| `LEAN_SYSROOT` | path | `null` | Active | Path to the Lean toolchain sysroot directory used during proof and spec checks |
| `MOCK_LEAN` | boolean | `null` | Active | Bypasses external Lean binary invocation during tests by enabling mock verification |
| `UALBF_ALLOW_LOGIC_MISMATCH` | boolean | `null` | Active | Allows execution to proceed despite logic or manifest hash mismatch during certificate verification |
| `UALBF_ALLOW_MISSING_SOURCES` | boolean | `null` | Active | Allows certificate verification without requiring local Lean 4 proof source files |
| `UALBF_ALLOW_UNVERIFIED_GPU` | boolean | `false` | Active | Allows execution of unverified GPU sieve algorithms and inclusion of GPU witness data |
| `UALBF_CERT_PATH` | path | `null` | Active | Custom path to the formal certificate JSON file for ingest and verification |
| `UALBF_DUMMY_PAPER_CI` | boolean | `null` | Active | Bypasses certificate verification during CI paper macro generation when set to 1 |
| `UALBF_IN_STAGING_WORKSPACE` | boolean | `null` | Active | Flag indicating execution within a staging workspace for audit checks |
| `UALBF_MAX_CERT_SIZE_MB` | float | `10.0` | Active | Maximum allowed formal certificate file size in megabytes |
| `UALBF_MAX_EXPONENT` | integer | `4` | Active | Maximum prime-power exponent considered in Phase 1 search space |
| `UALBF_MIN_RIGOR` | float | `0.0` | Active | Minimum acceptable rigor level ratio required during certificate verification |
| `UALBF_PREFIX_STOP_THRESHOLD` | integer | `100000000000` | Active | DFS search threshold where prefix construction stops when n_L exceeds this value |
| `UALBF_PROOF_MANIFEST` | path | `null` | Active | Custom path to the proof manifest JSON file containing theorem verification checksums |
| `UALBF_SIEVE_LIMIT` | integer | `250000` | Active | Number of primes evaluated in Phase 1 CRT tensor sieve |
| `UALBF_TARGET_MAX_LOG10` | integer | `37` | Active | Upper bound log10 exponent for search space (N < 10^max) |
| `UALBF_TARGET_MIN_LOG10` | integer | `35` | Active | Lower bound log10 exponent for search space (N > 10^min) |
| `UALBF_TRUSTED_PUBLIC_KEY` | string | `null` | Active | Hex-encoded Ed25519 public key pinned for formal certificate signature verification |
| `UALBF_ALLOW_MISSING_SOURCES` | boolean | `null` | Active | Bypasses missing source artifact checks during certificate verification when set to 1 |
| `ALLOW_UNVERIFIED_BUILD` | boolean | `null` | Deprecated | Deprecated bypass flag for unverified builds; execution is halted if detected |
| `UALBF_SKIP_VALIDATION` | boolean | `null` | Deprecated | Deprecated bypass flag for certificate validation; execution is halted if detected |

---
By explicitly defining these boundaries, future research contributors can better identify current verification gaps and contribute meaningful proofs to the repository.
