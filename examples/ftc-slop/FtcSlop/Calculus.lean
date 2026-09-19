namespace FtcSlop

open List

/-!
  This file is intentionally tiny and inelegant.  It is a from-scratch,
  executable model of the FTC over the discrete line:

    derivative f n = f (n + 1) - f n
    integral f a (b - a) = sum of derivative f on [a,b)

  There is no Mathlib import and no imported calculus theorem.  The point is
  to give Sieve a compact proof whose main result is easy to find.
-/

def derivative (f : Nat → Int) (n : Nat) : Int :=
  f (n + 1) - f n

def integral (f : Nat → Int) (a n : Nat) : Int :=
  (range n).foldl (fun previous k => previous + derivative f (a + k)) 0

theorem integral_zero (f : Nat → Int) (a : Nat) :
    integral f a 0 = 0 := by
  rfl

theorem integral_succ (f : Nat → Int) (a n : Nat) :
    integral f a (n + 1) = integral f a n + derivative f (a + n) := by
  simp [integral, List.range_succ, List.foldl_append]

theorem telescoping (f : Nat → Int) (a : Nat) :
    ∀ n, integral f a n = f (a + n) - f a := by
  intro n
  induction n with
  | zero =>
      simp [integral]
  | succ n ih =>
      rw [integral_succ, ih]
      calc
        f (a + n) - f a + derivative f (a + n) =
            (f (a + n) - f (a + n)) + (f (a + (n + 1)) - f a) := by
              simp only [derivative, Int.sub_eq_add_neg]
              ac_rfl
        _ = f (a + (n + 1)) - f a := by simp

/-- The fundamental theorem of calculus for this deliberately discrete model. -/
theorem fundamental_theorem_of_calculus
    (f : Nat → Int) (a b : Nat) (h : a ≤ b) :
    integral f a (b - a) = f b - f a := by
  have endpoint : a + (b - a) = b := Nat.add_sub_of_le h
  rw [telescoping]
  rw [endpoint]

theorem fundamental_theorem_of_calculus_at_zero
    (f : Nat → Int) (b : Nat) :
    integral f 0 b = f b - f 0 := by
  simpa using fundamental_theorem_of_calculus f 0 b (Nat.zero_le b)

end FtcSlop
