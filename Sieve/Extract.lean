import Lean
import Lean.Data.Json
import Lean.DeclarationRange
import Lean.DocString
import Lean.Meta.PPGoal
import Lean.Meta.Instances
import Lean.ProjFns
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

def binderInfoName : BinderInfo → String
  | .default => "explicit"
  | .implicit => "implicit"
  | .strictImplicit => "strictImplicit"
  | .instImplicit => "instanceImplicit"

structure ExpressionNode where
  id : Nat := 0
  kind : String
  children : Array Nat := #[]
  name : Option String := none
  index : Option Nat := none
  value : Option String := none
  binderInfo : Option String := none
  universeLevels : Array String := #[]
  deriving ToJson, Repr

structure ExpressionGraph where
  root : Nat
  nodes : Array ExpressionNode
  deriving ToJson, Repr

structure ExpressionGraphBuilder where
  nodes : Array ExpressionNode := #[]
  interned : Std.HashMap Expr Nat := {}

abbrev ExpressionGraphM := StateM ExpressionGraphBuilder

partial def internExpression (expr : Expr) : ExpressionGraphM Nat := do
  if let some id := (← get).interned[expr]? then
    return id

  let nodeWithoutId ← match expr with
    | .bvar index =>
        pure { kind := "boundVariable", index := some index : ExpressionNode }
    | .fvar id =>
        pure { kind := "freeVariable", name := some (toString id.name) : ExpressionNode }
    | .mvar id =>
        pure { kind := "metavariable", name := some (toString id.name) : ExpressionNode }
    | .sort level =>
        pure { kind := "sort", value := some (toString (repr level)) : ExpressionNode }
    | .const name levels =>
        pure {
          kind := "constant"
          name := some (toString name)
          universeLevels := levels.toArray.map (toString ∘ repr)
        }
    | .app function argument =>
        let functionId ← internExpression function
        let argumentId ← internExpression argument
        pure { kind := "application", children := #[functionId, argumentId] : ExpressionNode }
    | .lam binderName domain body binderInfo =>
        let domainId ← internExpression domain
        let bodyId ← internExpression body
        pure {
          kind := "lambda"
          children := #[domainId, bodyId]
          name := some (toString binderName)
          binderInfo := some (binderInfoName binderInfo)
        }
    | .forallE binderName domain body binderInfo =>
        let domainId ← internExpression domain
        let bodyId ← internExpression body
        pure {
          kind := "forall"
          children := #[domainId, bodyId]
          name := some (toString binderName)
          binderInfo := some (binderInfoName binderInfo)
        }
    | .letE binderName type value body nondep =>
        let typeId ← internExpression type
        let valueId ← internExpression value
        let bodyId ← internExpression body
        pure {
          kind := "let"
          children := #[typeId, valueId, bodyId]
          name := some (toString binderName)
          value := some (if nondep then "nondependent" else "dependent")
        }
    | .lit literal =>
        pure { kind := "literal", value := some (toString (repr literal)) : ExpressionNode }
    | .mdata _ body =>
        let bodyId ← internExpression body
        pure { kind := "metadata", children := #[bodyId] : ExpressionNode }
    | .proj structureName fieldIndex body =>
        let bodyId ← internExpression body
        pure {
          kind := "projection"
          children := #[bodyId]
          name := some (toString structureName)
          index := some fieldIndex
        }

  let state ← get
  let id := state.nodes.size
  let node := { nodeWithoutId with id }
  set ({
    nodes := state.nodes.push node
    interned := state.interned.insert expr id
  } : ExpressionGraphBuilder)
  return id

def expressionGraph (expr : Expr) : ExpressionGraph :=
  let (root, state) := (internExpression expr).run {}
  { root, nodes := state.nodes }

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
  isInternal : Bool
  isPrivate : Bool
  isUnsafe : Bool
  isPartial : Bool
  docString : Option String
  sourceRange : Option DeclarationRange
  levelParameters : Array String
  type : String
  hasValue : Bool
  typeStats : ExprStats
  valueStats : Option ExprStats
  typeGraph : ExpressionGraph
  valueGraph : Option ExpressionGraph
  statementDependencies : Array String
  proofDependencies : Array String
  axioms : Array String
  deriving ToJson, Repr

structure SymbolSnapshot where
  name : String
  kind : String
  moduleName : Option String
  isInternal : Bool
  isPrivate : Bool
  isUnsafe : Bool
  isPartial : Bool
  isClass : Bool
  isInstance : Bool
  isProjection : Bool
  deriving ToJson, Repr

structure ExtractionSnapshot where
  schemaVersion : Nat := 3
  leanVersion : String
  importedModule : String
  declarations : Array DeclarationSnapshot
  symbols : Array SymbolSnapshot
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
    isInternal := name.isInternal
    isPrivate := isPrivateName name
    isUnsafe := info.isUnsafe
    isPartial := info.isPartial
    docString
    sourceRange
    levelParameters := info.levelParams.toArray.map toString
    type := ← prettyPrintExpr info.type
    hasValue := value?.isSome
    typeStats := expressionStats info.type
    valueStats := value?.map expressionStats
    typeGraph := expressionGraph info.type
    valueGraph := value?.map expressionGraph
    statementDependencies := sortedNames info.type.getUsedConstantsAsSet
    proofDependencies := value?.map (sortedNames ·.getUsedConstantsAsSet) |>.getD #[]
    axioms := axioms.map toString |>.qsort (· < ·)
  }

def snapshotSymbol (name : Name) : MetaM SymbolSnapshot := do
  let env ← getEnv
  let some info := env.find? name
    | throwError "unknown referenced declaration '{name}'"
  return {
    name := toString name
    kind := declarationKind info
    moduleName := moduleForDeclaration? env name
    isInternal := name.isInternal
    isPrivate := isPrivateName name
    isUnsafe := info.isUnsafe
    isPartial := info.isPartial
    isClass := Lean.isClass env name
    isInstance := ← Meta.isInstance name
    isProjection := env.isProjectionFn name
  }

def declarationsInModule (env : Environment) (moduleName : Name) : Array Name :=
  match env.getModuleIdx? moduleName with
  | none => #[]
  | some moduleIdx =>
      let names := env.constants.toList.filterMap fun (name, _) =>
        if env.getModuleIdxFor? name == some moduleIdx then some name else none
      names.toArray.qsort fun left right => toString left < toString right

def parseName (value : String) : Name :=
  value.splitOn "." |>.foldl (fun name part => name ++ Name.mkSimple part) Name.anonymous

def coreContext : Core.Context := {
  fileName := "<sieve-extractor>"
  fileMap := FileMap.ofString ""
}

def extract (env : Environment) (targets : Array Name) : IO ExtractionSnapshot := do
  let action : MetaM (Array DeclarationSnapshot × Array SymbolSnapshot) := do
    let declarations ← targets.mapM snapshotDeclaration
    let mut referencedNames : NameSet := {}
    for target in targets do
      referencedNames := referencedNames.insert target
      if let some info := env.find? target then
        referencedNames := referencedNames ++ info.getUsedConstantsAsSet
    let sortedReferencedNames := referencedNames.toArray.qsort fun left right =>
      toString left < toString right
    let symbols ← sortedReferencedNames.mapM snapshotSymbol
    return (declarations, symbols)
  let ((declarations, symbols), _, _) ← action.toIO coreContext { env := env }
  return {
    leanVersion := Lean.versionString
    importedModule := "Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus"
    declarations
    symbols
  }

unsafe def extractMain (args : List String) : IO Unit := do
  initSearchPath (← findSysroot)
  let moduleName := `Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
  enableInitializersExecution
  let env ← importModules (loadExts := true) #[{ module := moduleName }] {}
  let targets := if args.isEmpty then declarationsInModule env moduleName else args.toArray.map parseName
  let snapshot ← extract env targets
  IO.println (toJson snapshot).compress

end Sieve

unsafe def main := Sieve.extractMain
