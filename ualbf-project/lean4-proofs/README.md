# UALBF Lean 4 Proof Library

Welcome to the Lean 4 formalized proof component of the Universal Arithmetic Lower Bound Framework (UALBF). 

This sub-project is designed to provide complete, machine-checked mathematical verification for the bounds and theorems used by the high-performance Rust engine. The purpose of this library is to guarantee that the mathematical constraints and pruning heuristics hardcoded into the computational search (such as the Prasad-Sunitha abundancy bounds and Legendre-Cattaneo obstructions) have rigorous foundational proofs.

## Architectural Overview

The Lean 4 project is structured in a **Four-Layer Architecture** strictly separating foundational logic from engine-specific computational wrappers. 

This strict separation ensures that purely mathematical theorems (Pure layer) do not depend on the specific hypotheses of Quasiperfect Numbers (QPN layer), and that the Rust engine's runtime checks (Engine and FFI layers) are directly backed by verified bounds.

## Directory and File Structure

Below is a comprehensive, file-by-file breakdown of the directory structure and the logical systems contained within each file.

```text
lean4-proofs/
├── UALBF.lean                # Root importer
└── UALBF/
    ├── Basic.lean            # Layer 0 definitions
    ├── FFI.lean              # C-bindings for the Rust engine
    ├── Pure/                 # Layer 1: General Mathematics (No QPN assumptions)
    ├── QPN/                  # Layer 2: Quasiperfect Number Theory
    └── Engine/               # Application Layer: Search Engine Formalization
```

### Root Files

- **`UALBF.lean`**
  The root import file for the entire Lean 4 proof library. This file simply sequentially imports the four foundational layers (Basic, Pure, QPN, and Engine/FFI). It ensures that when the Rust engine's `../rust-engine/build.rs` compiles the library, all mathematical dependencies are statically verified.

- **`UALBF/Basic.lean` (Layer 0)**
  Contains foundational definitions shared universally across all lower layers. Here, you will find the definitions for the arithmetic sum-of-divisors function `sigma`, the core predicate `IsQuasiperfect`, the `abundancy_index` formulation, `ExactValuation`, and the `Bipartition` structure which defines how the search space is mathematically split.

- **`UALBF/FFI.lean`**
  Computational wrappers bridging Lean 4's verified logic to the C-Foreign Function Interface (FFI). Because Lean erases pure `Prop` proofs at runtime, this file provides executable boolean `def`s (tagged with `@[export]`) that mirror the proven theorems. These definitions compile into the static library invoked directly by the Rust execution threads.

---

### Layer 1: Pure (`UALBF/Pure/`)
*Mathematical foundations independent of Quasiperfect Numbers. All proofs in this folder are "purely" structural and upstreamable, meaning they do not rely on the `IsQuasiperfect N` hypothesis.*

- **`UALBF/Pure/Arithmetic.lean`**
  Generic helpers and foundational lemmas handling parity, elementary modular arithmetic, and prime factorization properties. Serves as the bedrock for the more complex algebraic proofs upstream.

- **`UALBF/Pure/EulerProduct.lean`**
  Provides the algebraic decomposition of the Euler totient ratio ($N / \phi(N)$) into an abundancy index multiplied by a correction factor product. 

- **`UALBF/Pure/RationalBounds.lean`**
  A structural toolkit for evaluating strict bounds on rational functions and infinite series over primes. Used heavily to formalize constraints on prime abundancies.

- **`UALBF/Pure/Cyclotomic.lean`**
  The core machinery for cyclotomic polynomials. Contains evaluation bounds, characterization of primitive roots, GCD structure, and the vital Lifting-the-Exponent (LTE) lemma which is crucial for proving prime factor presence.

- **`UALBF/Pure/CyclotomicAlgebra.lean`**
  Provides auxiliary and inductive sub-lemmas bounding specific algebraic expansions required by the broader cyclotomic theory.
  
- **`UALBF/Pure/Zsigmondy.lean`**
  The formalization of the classical Zsigmondy's Theorem. This file elegantly combines the cyclotomic machinery to formally prove the existence of primitive prime divisors for the expression $p^n - 1$. 

---

### Layer 2: QPN (`UALBF/QPN/`)
*Domain-specific proofs modeling Quasiperfect Numbers. Every theorem in this folder requires the `h : IsQuasiperfect N` hypothesis.*

- **`UALBF/QPN/BasicProperties.lean`**
  Proves essential attributes that any Quasiperfect number must possess. Crucially, it proves that $\sigma(N)$ is odd, that QPNs must be perfect squares (specifically, odd perfect squares), and mathematically forbids QPNs from being double squares.

- **Obstruction.lean**
  Formalizes the **Universal Modulo-8 Obstruction** (Legendre-Cattaneo Filter). It proves that if $q$ is an odd prime factor of $\sigma(N)$ for a quasiperfect $N$, then $q \equiv 1$ or $3 \pmod 8$. 


- **`UALBF/QPN/PrasadSunitha.lean`**
  An extensive proof establishing the Prasad-Sunitha bound, specifically establishing that any QPN un-divisible by 3 and 5 ($\gcd(N, 15) = 1$) must possess a minimum of $\omega(N) \ge 15$ distinct prime factors.

- **`UALBF/QPN/AbundancyBound.lean`**
  Integrates the generalized Rational Bounds with the specific QPN hypothesis, cementing the cross-multiplied abundancy upper and lower bounds utilized throughout the paper's theory.

---

### Layer 3: Engine Verification (`UALBF/Engine/`)
*Contains the formal soundness proofs for the real-time search logic utilized within the Rust engine.*

- **`UALBF/Engine/Bipartition.lean`**
  Formalizes the Prefix-Suffix bipartition theorems utilized by the Rust backend's depth-first search. Proves the multiplicativity of the Sigma function over the bipartition space, formally validating that because left ($N_L$) and right ($N_R$) sub-trees are coprime, $\sigma(N_L \cdot N_R) = \sigma(N_L) \cdot \sigma(N_R)$.

- **`UALBF/Engine/SieveSoundness.lean`**
  Provides exact valuation theorems that formally prove the Rust engine's valuation sieve (which discards sub-topologies using divisibility invariants) is mathematically sound. It guarantees that the computational engine mathematically cannot miss a valid QPN.

---

## Active Proof Files Index

To ensure 100% layout coverage and prevent documentation drift, the following complete list of active Lean proof files in this sub-module are indexed and verified:

- `lakefile.lean`
- `UALBF.lean`
- `Validator.lean`
- `UALBF/Basic.lean`
- `UALBF/FFI.lean`
- `UALBF/FFI_generated.lean`
- `UALBF/BloomFilter.lean`
- `UALBF/ManifestConstants.lean`
- `UALBF/Pure/Arithmetic.lean`
- `UALBF/Pure/EulerProduct.lean`
- `UALBF/Pure/RationalBounds.lean`
- `UALBF/Pure/Cyclotomic.lean`
- `UALBF/Pure/CyclotomicAlgebra.lean`
- `UALBF/Pure/Zsigmondy.lean`
- `UALBF/Pure/Fixed64.lean`
- `UALBF/Pure/ABCConjecture.lean`
- `UALBF/QPN/BasicProperties.lean`
- `UALBF/QPN/Obstruction.lean`
- `UALBF/QPN/PrasadSunitha.lean`
- `UALBF/QPN/AbundancyBound.lean`
- `UALBF/QPN/TouchardQPN.lean`
- `UALBF/Engine/Bipartition.lean`
- `UALBF/Engine/SieveSoundness.lean`
- `UALBF/Engine/Mod5Bridge.lean`
- `UALBF/Engine/CyclotomicGraph.lean`
- `UALBF/Engine/SearchMonad.lean`
- `UALBF/Engine/TouchardBridge.lean`
- `UALBF/Engine/SearchState.lean`

