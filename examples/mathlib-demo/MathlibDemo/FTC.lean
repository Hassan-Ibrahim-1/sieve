import Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus

/-!
  A demo copy of the mathlib FTC proof shape.

  The point of this file is not to introduce a new theorem; it is to give Sieve
  a local proof body with real structure instead of a one-line wrapper around
  mathlib's final theorem.
-/

open MeasureTheory Set intervalIntegral

namespace MathlibDemo.FTC

variable {a b : ℝ}
variable {E : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E] [CompleteSpace E]
variable {f f' : ℝ → E}

/-- FTC, right-derivative version on an ordered interval. -/
theorem integral_eq_sub_of_hasDeriv_right_of_le_demo
    (hab : a ≤ b)
    (hcont : ContinuousOn f (Icc a b))
    (hderiv : ∀ x ∈ Ioo a b, HasDerivWithinAt f (f' x) (Ioi x) x)
    (f'int : IntervalIntegrable f' volume a b) :
    ∫ y in a..b, f' y = f b - f a := by
  refine (SeparatingDual.eq_iff_forall_dual_eq (R := ℝ)).2 fun g => ?_
  rw [← g.intervalIntegral_comp_comm f'int, g.map_sub]
  exact intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le_real hab
    (g.continuous.comp_continuousOn hcont)
    (fun x hx => g.hasFDerivAt.comp_hasDerivWithinAt x (hderiv x hx))
    (g.integrable_comp ((intervalIntegrable_iff_integrableOn_Icc_of_le hab enorm_ne_top).1 f'int))

/-- FTC, right-derivative version on an unordered interval. -/
theorem integral_eq_sub_of_hasDeriv_right_demo
    (hcont : ContinuousOn f (uIcc a b))
    (hderiv : ∀ x ∈ Ioo (min a b) (max a b), HasDerivWithinAt f (f' x) (Ioi x) x)
    (hint : IntervalIntegrable f' volume a b) :
    ∫ y in a..b, f' y = f b - f a := by
  rcases le_total a b with hab | hab
  · simp only [uIcc_of_le, min_eq_left, max_eq_right, hab] at hcont hderiv hint
    apply integral_eq_sub_of_hasDeriv_right_of_le_demo hab hcont hderiv hint
  · simp only [uIcc_of_ge, min_eq_right, max_eq_left, hab] at hcont hderiv
    rw [integral_symm, integral_eq_sub_of_hasDeriv_right_of_le_demo hab hcont hderiv hint.symm,
      neg_sub]

/-- FTC, ordinary derivative version on an ordered interval. -/
theorem integral_eq_sub_of_hasDerivAt_of_le_demo
    (hab : a ≤ b)
    (hcont : ContinuousOn f (Icc a b))
    (hderiv : ∀ x ∈ Ioo a b, HasDerivAt f (f' x) x)
    (hint : IntervalIntegrable f' volume a b) :
    ∫ y in a..b, f' y = f b - f a :=
  integral_eq_sub_of_hasDeriv_right_of_le_demo hab hcont
    (fun x hx => (hderiv x hx).hasDerivWithinAt) hint

/-- FTC, ordinary derivative version on an unordered interval. -/
theorem fundamental_theorem_of_calculus_demo
    (hderiv : ∀ x ∈ uIcc a b, HasDerivAt f (f' x) x)
    (hint : IntervalIntegrable f' volume a b) :
    ∫ y in a..b, f' y = f b - f a :=
  integral_eq_sub_of_hasDeriv_right_demo
    (HasDerivAt.continuousOn hderiv)
    (fun _x hx => (hderiv _ (mem_Icc_of_Ioo hx)).hasDerivWithinAt)
    hint

end MathlibDemo.FTC
