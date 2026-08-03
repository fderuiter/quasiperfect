import Mathlib.Data.Nat.Basic
import Mathlib.Tactic.Ring
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.NormNum
import Mathlib.Tactic.IntervalCases

lemma q_sq_le_two_pow (q : ℕ) (hq : 7 ≤ q) : q ^ 2 ≤ 2 ^ (q - 1) := by
  induction q, hq using Nat.le_induction with
  | base => norm_num
  | succ k _hk ih =>
    have : (k + 1) ^ 2 = k ^ 2 + 2 * k + 1 := by ring
    have hz : 2 * k + 1 ≤ k ^ 2 := by nlinarith
    calc (k + 1) ^ 2
      _ ≤ k ^ 2 + k ^ 2 := Nat.add_le_add_left hz _
      _ = 2 * k ^ 2 := by ring
      _ ≤ 2 * 2 ^ (k - 1) := Nat.mul_le_mul_left 2 ih
      _ = 2 ^ k := by rw [← pow_one 2, ← pow_add]; congr 1; omega

lemma composite_bound_p_q (p q : ℕ) (hp : 3 ≤ p) (h_lt : p < q) :
    p * q * 2 ^ p + 2 ^ (q + 1) ≤ 2 ^ (2 * q) := by
  by_cases hq7 : q < 7
  · interval_cases q
    · exfalso; revert hp h_lt; decide
    · exfalso; revert hp h_lt; decide
    · exfalso; revert hp h_lt; decide
    · interval_cases p <;> decide
    · interval_cases p <;> decide
  · have hq7_ge : 7 ≤ q := by omega
    have hpq : p * q ≤ q ^ 2 := by nlinarith
    have hq2 : q ^ 2 ≤ 2 ^ (q - 1) := q_sq_le_two_pow q hq7_ge
    have h_pow_p : 2 ^ p ≤ 2 ^ (q - 1) := Nat.pow_le_pow_right (by omega) (by omega)
    have h_part1 : p * q * 2 ^ p ≤ 2 ^ (2 * q - 2) := by
      calc p * q * 2 ^ p
        _ ≤ q ^ 2 * 2 ^ p := Nat.mul_le_mul_right _ hpq
        _ ≤ 2 ^ (q - 1) * 2 ^ p := Nat.mul_le_mul_right _ hq2
        _ ≤ 2 ^ (q - 1) * 2 ^ (q - 1) := Nat.mul_le_mul_left _ h_pow_p
        _ = 2 ^ (2 * q - 2) := by rw [← pow_add]; congr 1; omega
    have h_part2 : 2 ^ (q + 1) ≤ 2 ^ (2 * q - 2) := Nat.pow_le_pow_right (by omega) (by omega)
    calc p * q * 2 ^ p + 2 ^ (q + 1)
      _ ≤ 2 ^ (2 * q - 2) + 2 ^ (2 * q - 2) := Nat.add_le_add h_part1 h_part2
      _ = 2 * 2 ^ (2 * q - 2) := by ring
      _ = 2 ^ (2 * q - 1) := by rw [← pow_one 2, ← pow_add]; congr 1; omega
      _ ≤ 2 ^ (2 * q) := Nat.pow_le_pow_right (by omega) (by omega)
