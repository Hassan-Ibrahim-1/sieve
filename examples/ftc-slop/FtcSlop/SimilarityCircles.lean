namespace FtcSlop.SimilarityCircles

/-!
This module is a deliberately small UI fixture for Sieve's equivalence-group
rendering. It contains three clusters of same-kind declarations with exact or
alpha-equivalent statements. Each cluster should be surrounded by its own
translucent circle in the graph.

The proofs intentionally refer across clusters so the example also exercises
dependency arrows whose endpoints are equivalence groups.
-/

-- Group 1: three alpha-equivalent identity theorems.
theorem identityFirst (P : Prop) (h : P) : P := h

theorem identitySecond (Q : Prop) (proof : Q) : Q := proof

theorem identityThird (R : Prop) (evidence : R) : R := evidence

-- Group 2: three equivalent conjunction-introduction theorems. Two proofs
-- depend on members of the identity group above.
theorem conjunctionDirect (P Q : Prop) (hP : P) (hQ : Q) : P ∧ Q :=
  ⟨hP, hQ⟩

theorem conjunctionUsingIdentity (A B : Prop) (hA : A) (hB : B) : A ∧ B :=
  ⟨identityFirst A hA, identitySecond B hB⟩

theorem conjunctionUsingThird (X Y : Prop) (hX : X) (hY : Y) : X ∧ Y :=
  ⟨identityThird X hX, identityThird Y hY⟩

-- Group 3: two equivalent double-negation introduction theorems. The second
-- proof also depends on the identity group.
theorem doubleNegationDirect (P : Prop) (h : P) : ¬¬P := by
  intro notP
  exact notP h

theorem doubleNegationUsingIdentity (Q : Prop) (proof : Q) : ¬¬Q := by
  intro notQ
  exact notQ (identityFirst Q proof)

-- A singleton result connects the conjunction and double-negation groups.
theorem combinedResult (P Q : Prop) (hP : P) (hQ : Q) : ¬¬(P ∧ Q) :=
  doubleNegationUsingIdentity (P ∧ Q) (conjunctionUsingIdentity P Q hP hQ)

end FtcSlop.SimilarityCircles
