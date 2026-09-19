import Lean
import Lean.Data.Json
import Lean.DeclarationRange
import Lean.DocString
import Lean.Meta.PPGoal
import Lean.Util.CollectAxioms
import Lean.Util.FoldConsts

open Lean

namespace Sieve

structure ExprStats where
  nodes : Nat := 0
  maxDepth : Nat := 0
  boundVariables : Nat := 0
  freeVariables : Nat := 0
  metavariables : Nat := 0
  sorts : Nat := 0
  constants : Nat := 0
  applications : Nat := 0
  lambdas : Nat := 0
  foralls : Nat := 0
  lets : Nat := 0
  literals : Nat := 0
  metadata : Nat := 0
  projections : Nat := 0
  deriving Inhabited, ToJson, Repr

def ExprStats.combine (left right : ExprStats) : ExprStats :=
  { nodes := left.nodes + right.nodes
    maxDepth := max left.maxDepth right.maxDepth
    boundVariables := left.boundVariables + right.boundVariables
    freeVariables := left.freeVariables + right.freeVariables
    metavariables := left.metavariables + right.metavariables
    sorts := left.sorts + right.sorts
    constants := left.constants + right.constants
    applications := left.applications + right.applications
    lambdas := left.lambdas + right.lambdas
    foralls := left.foralls + right.foralls
    lets := left.lets + right.lets
    literals := left.literals + right.literals
    metadata := left.metadata + right.metadata
    projections := left.projections + right.projections }

partial def expressionStats (expr : Expr) (depth : Nat := 1) : ExprStats :=
  let here : ExprStats := { nodes := 1, maxDepth := depth }
  match expr with
  | .bvar _ => { here with boundVariables := 1 }
  | .fvar _ => { here with freeVariables := 1 }
  | .mvar _ => { here with metavariables := 1 }
  | .sort _ => { here with sorts := 1 }
  | .const _ _ => { here with constants := 1 }
  | .app fn arg =>
      { here with applications := 1 }
        |>.combine (expressionStats fn (depth + 1))
        |>.combine (expressionStats arg (depth + 1))
  | .lam _ domain body _ =>
      { here with lambdas := 1 }
        |>.combine (expressionStats domain (depth + 1))
        |>.combine (expressionStats body (depth + 1))
  | .forallE _ domain body _ =>
      { here with foralls := 1 }
        |>.combine (expressionStats domain (depth + 1))
        |>.combine (expressionStats body (depth + 1))
  | .letE _ type value body _ =>
      { here with lets := 1 }
        |>.combine (expressionStats type (depth + 1))
        |>.combine (expressionStats value (depth + 1))
        |>.combine (expressionStats body (depth + 1))
  | .lit _ => { here with literals := 1 }
  | .mdata _ body =>
      { here with metadata := 1 }
        |>.combine (expressionStats body (depth + 1))
  | .proj _ _ body =>
      { here with projections := 1 }
        |>.combine (expressionStats body (depth + 1))

def declarationKind : ConstantInfo → String
  | .axiomInfo _ => "axiom"
  | .defnInfo _ => "definition"
  | .thmInfo _ => "theorem"
  | .opaqueInfo _ => "opaque"
  | .quotInfo _ => "quotient"
  | .inductInfo _ => "inductive"
  | .ctorInfo _ => "constructor"
  | .recInfo _ => "recursor"

def sortedNames (names : NameSet) : Array String :=
  names.toArray.map toString |>.qsort (· < ·)

instance : ToJson Position where
  toJson position := json% {
    line: $(position.line),
    column: $(position.column)
  }

instance : ToJson DeclarationRange where
  toJson range := json% {
    start: $(toJson range.pos),
    "end": $(toJson range.endPos)
  }

structure DeclarationSnapshot where
  name : String
  kind : String
  moduleName : Option String
  docString : Option String
  sourceRange : Option DeclarationRange
  levelParameters : Array String
  type : String
  hasValue : Bool
  typeStats : ExprStats
  valueStats : Option ExprStats
  statementDependencies : Array String
  proofDependencies : Array String
  axioms : Array String
  deriving ToJson, Repr

structure ExtractionSnapshot where
  schemaVersion : Nat := 1
  importedModule : String
  declarations : Array DeclarationSnapshot
  deriving ToJson, Repr

def prettyPrintExpr (expr : Expr) : MetaM String := do
  return toString (← Meta.ppExpr expr)

def moduleForDeclaration? (env : Environment) (name : Name) : Option String := do
  let moduleIdx ← env.getModuleIdxFor? name
  let moduleName ← env.allImportedModuleNames[moduleIdx.toNat]?
  return toString moduleName

def snapshotDeclaration (name : Name) : MetaM DeclarationSnapshot := do
  let env ← getEnv
  let some info := env.find? name
    | throwError "unknown declaration '{name}'"
  let value? := info.value? true
  let axioms ← Lean.collectAxioms name
  let docString ← Lean.findDocString? env name
  let sourceRange := (← Lean.findDeclarationRanges? name).map (·.range)
  return {
    name := toString name
    kind := declarationKind info
    moduleName := moduleForDeclaration? env name
    docString
    sourceRange
    levelParameters := info.levelParams.toArray.map toString
    type := ← prettyPrintExpr info.type
    hasValue := value?.isSome
    typeStats := expressionStats info.type
    valueStats := value?.map expressionStats
    statementDependencies := sortedNames info.type.getUsedConstantsAsSet
    proofDependencies := value?.map (sortedNames ·.getUsedConstantsAsSet) |>.getD #[]
    axioms := axioms.map toString |>.qsort (· < ·)
  }

def defaultTargets : Array Name := #[
  `intervalIntegral.integral_deriv_eq_sub,
  `intervalIntegral.integral_deriv_eq_sub',
  `intervalIntegral.integral_deriv_eq_sub_uIoo
]

def parseName (value : String) : Name :=
  value.splitOn "." |>.foldl (fun name part => name ++ Name.mkSimple part) Name.anonymous

def coreContext : Core.Context := {
  fileName := "<sieve-extractor>"
  fileMap := FileMap.ofString ""
}

def extract (env : Environment) (targets : Array Name) : IO ExtractionSnapshot := do
  let action : MetaM (Array DeclarationSnapshot) := targets.mapM snapshotDeclaration
  let (declarations, _, _) ← action.toIO coreContext { env := env }
  return {
    importedModule := "Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus"
    declarations
  }

def extractMain (args : List String) : IO Unit := do
  initSearchPath (← findSysroot)
  let moduleName := `Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
  let env ← importModules #[{ module := moduleName }] {}
  let targets := if args.isEmpty then defaultTargets else args.toArray.map parseName
  let snapshot ← extract env targets
  IO.println (toJson snapshot).compress

end Sieve

def main := Sieve.extractMain
