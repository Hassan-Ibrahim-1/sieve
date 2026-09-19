import Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus

namespace Sieve.ProofStepFixtures

theorem forwardLeft (P Q : Prop) (h : P) (f : P → Q) : Q := f h

theorem forwardRight (P R : Prop) (h : P) (f : P → R) : R := f h

/-- A local fact is deliberately reused by two later named applications. -/
theorem reusedFact (P Q R : Prop) (hP : P) (hPQ : P → Q) (hPR : P → R) : Q ∧ R := by
  have shared : P := hP
  exact ⟨forwardLeft P Q shared hPQ, forwardRight P R shared hPR⟩

/-- Branches establish identical-looking propositions in different scopes. -/
theorem branchScopes (P : Prop) [Decidable P] : P ∨ ¬P := by
  by_cases h : P
  · exact Or.inl h
  · exact Or.inr h

/-- The same proposition is established by two distinct local proof terms. -/
theorem sameClaimDifferentArguments (P : Prop) (first second : P) : P ∧ P := by
  have fromFirst : P := first
  have fromSecond : P := id second
  exact ⟨fromFirst, fromSecond⟩

end Sieve.ProofStepFixtures
