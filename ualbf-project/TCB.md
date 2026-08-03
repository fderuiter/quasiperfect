# Trusted Computing Base (TCB) & Verification Boundaries

This document defines the Trusted Computing Base (TCB) for the Unified Algebraic-Lattice Bipartition Framework (UALBF). To maintain transparency and strict mathematical certitude, we explicitly disclose the boundaries of our formally verified claims. The components listed below act as unverified external blocks, FFI boundaries, or trusted mathematical assumptions rather than mechanically checked proofs.

## 1. Lean-to-Rust FFI Boundary
The Foreign Function Interface (FFI) bridging the Lean 4 formalization and the Rust execution engine is unverified.
- **Current State:** The Rust execution engine relies on C-compatible data serialization and exported semantics via Lean's `@[export]` pragmas.
- **Verification Status:** While the individual Lean 4 proofs are mechanically checked and the Rust execution logic is highly robust, the bridging logic across the boundary itself forms a critical part of the TCB and is not formally proven.

## 2. GPU Pollard's Rho Pipeline (Inactive)
The repository contains a highly parallelized batch-factorization GPU Pollard's Rho pipeline, implemented in Apple Metal (`rust-engine/src/unverified/gpu.rs`).
- **Current State:** This pipeline is completely bypassed in the active paths. High-performance execution relies entirely on sequential CPU loops.
- **Verification Status:** The GPU pipeline operations are unverified. They are not active during the main verified search processes and form no part of the end-to-end verification claims.

## 3. Bloom Filter Hashing Primitives
The Bloom filter's wrapping double-hashing logic is formally verified in Lean 4 to have zero false negatives. However, the underlying cryptographic (SHA-256) and multiplicative (FNV-1a) hash primitives that generate the initial hash seeds are excluded from formal verification.
- **Current State:** The Lean 4 formalization guarantees that the index generation step maps inputs securely to the bitset, but relies on Rust-side unverified implementations of SHA-256 and FNV-1a.
- **Verification Status:** The hash primitives themselves form part of the TCB and remain unverified.

## 4. Miller-Rabin Verification Boundaries
The verification pipeline does not treat the 20-base Miller-Rabin sufficiency test as an active, trusted mathematical axiom. To preserve performance guarantees while maintaining complete mathematical certitude and transparency, the system configuration manifest and verification layer reject the assumption that any 20-base probabilistic check is axiomatically sufficient for primality. This is formally represented by the spec function "lean_miller_rabin_20_base_sufficiency" and the system bounds manifest `bounds_manifest.json`, where the "is_axiomatic" status is set to `false`.

Instead of probabilistic sufficiency assumptions, the framework employs a hybrid tiered primality pipeline in the `verified_is_prime` function:

- **Inputs Below 2^64 (Smaller Candidate Primes):**
  The engine routes these inputs through a proven, deterministic 12-base Miller-Rabin test. The 12 proven bases used are: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, and 37. Proving the deterministic correctness of these 12 bases within this domain is mathematically established and trusted.
  
- **Inputs Equal to or Exceeding 2^64 (Larger Candidate Primes):**
  Inputs at or above this boundary cannot be verified solely using probabilistic Miller-Rabin checks. Instead, they are subjected to a rigorous certificate-backed verification pathway. The 20-base Miller-Rabin check is used strictly as a fast, non-binding pre-filter to reject composite candidates. Any candidate that passes this pre-filter must be validated using a mathematically rigorous, verified Pocklington certificate via `generate_and_verify_pocklington` for absolute certitude. This certificate-backed pathway is the mandatory mechanism for all inputs equal to or exceeding 2^64.

---
By explicitly defining these boundaries, future research contributors can better identify current verification gaps and contribute meaningful proofs to the repository.
