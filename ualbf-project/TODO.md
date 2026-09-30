# UALBF Project Status and Open Work

Open work is tracked in [GitHub issues](https://github.com/fderuiter/quasiperfect/issues). This file only summarizes where the project stands.

**Toolchain:** <!-- TOOLCHAIN_ENV_START -->leanprover/lean4:v4.30.0<!-- TOOLCHAIN_ENV_END --> (`lean-toolchain` pins <!-- TOOLCHAIN_VERSION_START -->v4.30.0<!-- TOOLCHAIN_VERSION_END -->), with Mathlib pinned in `lake-manifest.json`.

## What is established

- The Lean library builds with warnings as errors. Every theorem registered in `proof_manifest.json` is proven, and no axioms are whitelisted (see `TCB.md` §4a).
- Proven results about a quasiperfect number N include:
  - N is an odd perfect square.
  - N satisfies the mod-8 and mod-24 (Touchard) residue constraints on σ(N).
  - If gcd(N, 15) = 1, then ω(N) ≥ 15.
  - If gcd(N, 3) = 1, then ω(N) ≥ 7.
  - The cyclotomic and Zsigmondy forcing lemmas about primes dividing σ(N).

## What is not established

- **No lower bound on N.** The search engine is a prototype. It only considers primes below 250,000 with exponent 2e ≤ 8 (#565). Its CDG "forced cascade" prune is unsound (#566). Several pruning rules rest on assumptions that are not proven, and trace events label each prune with the theorem behind it or the assumption it makes.
- The current literature bounds are N > 10⁴⁵ (Alekseyev 2026) and ω(N) ≥ 7 (Hagis & Cohen 1982). An ω(N) ≥ 8 claim (Toyohara–Tao–Yao 2026) is still conditional.

## Direction

The plan is a Lean-checked certificate: an untrusted search emits a proof tree, and a small verified checker validates it. See the epic #569 and its milestones #570–#574.

The April 2026 remediation checklist that used to live in this file is complete. It remains in the git history.
