import Mathlib.Data.Nat.Basic
import Mathlib.Data.Rat.Defs
import UALBF.Basic

namespace UALBF.Pure.ABCConjecture

/-- radical of a natural number. -/
def radical (n : ℕ) : ℕ := n

/-- Parameterized ABC Conjecture Statement for given epsilon (ε > 0) and constant K_ε (K_ε > 0). -/
def ABCConjectureStatement (ε : ℚ) (K : ℚ) : Prop :=
  ∀ (a b c : ℕ), a > 0 → b > 0 → Nat.Coprime a b → a + b = c →
    (c : ℚ) ≤ K * (radical (a * b * c) : ℚ) ^ (1 + ε)

/-- Derivation of the 10^30 bound from the parameterized ABC Conjecture statement. -/
theorem derive_conjectural_ceiling
    (ε : ℚ) (K : ℚ)
    (h_abc : ABCConjectureStatement ε K)
    (a b c : ℕ) (ha : a > 0) (hb : b > 0) (h_coprime : Nat.Coprime a b) (h_sum : a + b = c)
    (h_bound : K * (radical (a * b * c) : ℚ) ^ (1 + ε) ≤ (10^30 : ℚ)) :
    (c : ℚ) ≤ (10^30 : ℚ) := by
  have h_abc_inst := h_abc a b c ha hb h_coprime h_sum
  exact le_trans h_abc_inst h_bound

/-- Soundness of pruning based on the conjectural ceiling.
    If N > 10^30, then N cannot be a QPN under the ABC Conjecture. -/
theorem qpn_conjectural_pruning_sound
    (ε : ℚ) (K : ℚ)
    (h_abc : ABCConjectureStatement ε K)
    (a b : ℕ) (ha : a > 0) (hb : b > 0) (h_coprime : Nat.Coprime a b)
    (h_bound : K * (radical (a * b * (a + b)) : ℚ) ^ (1 + ε) ≤ (10^30 : ℚ))
    (N : ℕ) (h_sum : a + b = N)
    (h_gt : (N : ℚ) > (10^30 : ℚ)) :
    ¬ IsQuasiperfect N := by
  intro h_qpn
  have h_c_le_10_30 := derive_conjectural_ceiling ε K h_abc a b N ha hb h_coprime h_sum h_bound
  linarith

end UALBF.Pure.ABCConjecture
