import Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus

open MeasureTheory Set

namespace Sieve.SlopCorpus

variable {E : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E] [CompleteSpace E]
variable {f : ℝ → E} {a b : ℝ}

/-- A direct generated-looking copy with a neutral name. -/
theorem resultOne (hderiv : ∀ x ∈ Set.uIcc a b, DifferentiableAt ℝ f x)
    (hint : IntervalIntegrable (deriv f) volume a b) :
    ∫ y in a..b, deriv f y = f b - f a := by
  have mainResult := intervalIntegral.integral_deriv_eq_sub hderiv hint
  have copiedResult : ∫ y in a..b, deriv f y = f b - f a := mainResult
  clear mainResult
  exact copiedResult

/-- The same statement produced with an avoidable symmetry round trip. -/
theorem resultTwo (hderiv : ∀ x ∈ Set.uIcc a b, DifferentiableAt ℝ f x)
    (hint : IntervalIntegrable (deriv f) volume a b) :
    ∫ y in a..b, deriv f y = f b - f a := by
  have mainResult := intervalIntegral.integral_deriv_eq_sub hderiv hint
  have backwards : f b - f a = ∫ y in a..b, deriv f y := mainResult.symm
  clear mainResult
  exact backwards.symm

/-- The same statement again, this time hidden behind a local proposition. -/
theorem resultThree (hderiv : ∀ x ∈ Set.uIcc a b, DifferentiableAt ℝ f x)
    (hint : IntervalIntegrable (deriv f) volume a b) :
    ∫ y in a..b, deriv f y = f b - f a := by
  let claimed : Prop := ∫ y in a..b, deriv f y = f b - f a
  have mainResult : claimed := by
    change ∫ y in a..b, deriv f y = f b - f a
    exact intervalIntegral.integral_deriv_eq_sub hderiv hint
  change claimed
  exact mainResult

end Sieve.SlopCorpus
