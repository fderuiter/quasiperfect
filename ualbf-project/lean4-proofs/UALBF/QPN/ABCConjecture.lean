import UALBF.Basic
import UALBF.Pure.ABCConjecture

/-!
# QPN Conjectural ABC Module

Connects `IsQuasiperfect N` with the pure ABC conjectural bounds derived in `UALBF.Pure.ABCConjecture`.
-/

namespace UALBF.QPN.ABCConjecture

open UALBF
open UALBF.Pure.ABCConjecture

/-- Soundness of QPN pruning based on the conjectural ceiling.
    If N > 10^30, then N cannot be a QPN under the ABC Conjecture. -/
theorem qpn_conjectural_pruning_sound
    (ε : ℚ) (K : ℚ)
    (h_abc : ABCConjectureStatement ε K)
    (a b : ℕ) (ha : a > 0) (hb : b > 0) (h_coprime : Nat.Coprime a b)
    (h_bound : K ^ (1 + ε).den * (radical (a * b * (a + b)) : ℚ) ^ (1 + ε).num.natAbs ≤ (10^30 : ℚ) ^ (1 + ε).den)
    (N : ℕ) (h_sum : a + b = N)
    (h_gt : (N : ℚ) ^ (1 + ε).den > (10^30 : ℚ) ^ (1 + ε).den)
    (_h_qpn : IsQuasiperfect N) : False := by
  exact conjectural_ceiling_size_exclusion ε K h_abc a b ha hb h_coprime h_bound N h_sum h_gt

/-- Non-existence of QPN above the conjectural ceiling. -/
theorem qpn_conjectural_pruning_not_qpn
    (ε : ℚ) (K : ℚ)
    (h_abc : ABCConjectureStatement ε K)
    (a b : ℕ) (ha : a > 0) (hb : b > 0) (h_coprime : Nat.Coprime a b)
    (h_bound : K ^ (1 + ε).den * (radical (a * b * (a + b)) : ℚ) ^ (1 + ε).num.natAbs ≤ (10^30 : ℚ) ^ (1 + ε).den)
    (N : ℕ) (h_sum : a + b = N)
    (h_gt : (N : ℚ) ^ (1 + ε).den > (10^30 : ℚ) ^ (1 + ε).den) :
    ¬ IsQuasiperfect N := by
  intro h_qpn
  exact qpn_conjectural_pruning_sound ε K h_abc a b ha hb h_coprime h_bound N h_sum h_gt h_qpn

end UALBF.QPN.ABCConjecture
