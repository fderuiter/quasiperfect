import UALBF.Basic
import Mathlib.Data.Nat.Basic
import UALBF.FFI

namespace UALBF.Engine.Mod1155Bridge

open UALBF.FFI

theorem mod_eq_of_mod_eq_of_dvd {a b m d : ℕ} (h_eq : a % m = b % m) (hd : d ∣ m) : a % d = b % d := by
  rcases hd with ⟨k, rfl⟩
  have h_left : a % (d * k) % d = a % d := Nat.mod_mul_left_mod a d k
  have h_right : b % (d * k) % d = b % d := Nat.mod_mul_left_mod b d k
  rw [←h_left, h_eq, h_right]

theorem mod1155_to_mod3 (z v : ℕ) (h : z^2 % 1155 = v % 1155) : z^2 % 3 = v % 3 := by
  have hd : 3 ∣ 1155 := by decide
  exact mod_eq_of_mod_eq_of_dvd h hd

theorem mod1155_to_mod5 (z v : ℕ) (h : z^2 % 1155 = v % 1155) : z^2 % 5 = v % 5 := by
  have hd : 5 ∣ 1155 := by decide
  exact mod_eq_of_mod_eq_of_dvd h hd

theorem mod1155_to_mod7 (z v : ℕ) (h : z^2 % 1155 = v % 1155) : z^2 % 7 = v % 7 := by
  have hd : 7 ∣ 1155 := by decide
  exact mod_eq_of_mod_eq_of_dvd h hd

theorem mod1155_to_mod11 (z v : ℕ) (h : z^2 % 1155 = v % 1155) : z^2 % 11 = v % 11 := by
  have hd : 11 ∣ 1155 := by decide
  exact mod_eq_of_mod_eq_of_dvd h hd

theorem mod1155_soundness (z v : ℕ) (h : z^2 % 1155 = v % 1155) :
  z^2 % 3 = v % 3 ∧ z^2 % 5 = v % 5 ∧ z^2 % 7 = v % 7 ∧ z^2 % 11 = v % 11 := by
  exact ⟨mod1155_to_mod3 z v h, mod1155_to_mod5 z v h, mod1155_to_mod7 z v h, mod1155_to_mod11 z v h⟩

theorem ualbf_check_crt_1155_sound (z_val x_l_val : U512)
  (h_prune : ualbf_check_crt_1155_impl z_val x_l_val = false) :
  (fromU512 z_val) ^ 2 % 1155 ≠ (fromU512 x_l_val) % 1155 := by
  unfold ualbf_check_crt_1155_impl at h_prune
  intro h_eq
  have h3 := mod1155_to_mod3 (fromU512 z_val) (fromU512 x_l_val) h_eq
  have h5 := mod1155_to_mod5 (fromU512 z_val) (fromU512 x_l_val) h_eq
  have h7 := mod1155_to_mod7 (fromU512 z_val) (fromU512 x_l_val) h_eq
  have h11 := mod1155_to_mod11 (fromU512 z_val) (fromU512 x_l_val) h_eq
  simp only [h3, h5, h7, h11, decide_true, Bool.and_true, Bool.and_self] at h_prune

end UALBF.Engine.Mod1155Bridge
