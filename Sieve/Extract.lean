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

structure ProofContextEntry where
  id : String
  userName : String
  kind : String
  binderInfo : String
  type : String
  typeGraph : ExpressionGraph
  value : Option String := none
  deriving ToJson, Repr

structure ProofStep where
  id : Nat
  kind : String
  proposition : String
  propositionGraph : ExpressionGraph
  context : Array ProofContextEntry
  scope : Array String
  proofTermPath : Array Nat
  prerequisiteSteps : Array Nat := #[]
  hypothesisReferences : Array String := #[]
  namedReferences : Array String := #[]
  deriving ToJson, Repr

structure NamedResultSnapshot where
  name : String
  kind : String
  moduleName : Option String
  type : String
  typeGraph : ExpressionGraph
  deriving ToJson, Repr

structure ProofStepExtraction where
  complete : Bool := true
  truncationReason : Option String := none
  visitedTerms : Nat := 0
  conclusionStep : Option Nat := none
  steps : Array ProofStep := #[]
  namedResults : Array NamedResultSnapshot := #[]
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
  generationKind : Option String
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
  proofSteps : Option ProofStepExtraction := none
  deriving ToJson, Repr

structure SymbolSnapshot where
  name : String
  kind : String
  moduleName : Option String
  generationKind : Option String
  isInternal : Bool
  isPrivate : Bool
  isUnsafe : Bool
  isPartial : Bool
  isClass : Bool
  isInstance : Bool
  isProjection : Bool
  deriving ToJson, Repr

structure ExtractionSnapshot where
  schemaVersion : Nat := 6
  leanVersion : String
  importedModules : Array String
  declarations : Array DeclarationSnapshot
  symbols : Array SymbolSnapshot
  deriving ToJson, Repr

def prettyPrintExpr (expr : Expr) : MetaM String := do
  return toString (← Meta.ppExpr expr)

def moduleForDeclaration? (env : Environment) (name : Name) : Option String := do
  let moduleIdx ← env.getModuleIdxFor? name
  let moduleName ← env.allImportedModuleNames[moduleIdx.toNat]?
  return toString moduleName

/--
Classify compiler-created declarations using Lean metadata. Private declarations
are deliberately not classified as generated: privacy and provenance are
independent properties, even though the default analysis filter hides both.
-/
def generationKind? (env : Environment) (name : Name) (info : ConstantInfo) : MetaM (Option String) := do
  -- `eqnsExt` is non-persistent. Prime it from the possible parent so imported
  -- equation theorems are recognized just like declarations in the current file.
  discard <| Meta.getEqnsFor? name.getPrefix
  if ← Meta.isEqnThm name then
    return some "equationTheorem"
  if ← Meta.isMatcher name then
    return some "matcher"
  if isNoConfusion env name then
    return some "noConfusion"
  if isAuxRecursor env name then
    return some "auxiliaryRecursor"
  if (match info with | .recInfo _ => true | _ => false) then
    return some "recursor"
  if env.isProjectionFn name then
    return some "projection"
  if name.isInternal then
    return some "internalName"
  return none

def parseName (value : String) : Name :=
  value.splitOn "." |>.foldl (fun name part => name ++ Name.mkSimple part) Name.anonymous

structure ProofStepBuilder where
  steps : Array ProofStep := #[]
  visitedTerms : Nat := 0
  complete : Bool := true
  truncationReason : Option String := none

def maxProofSteps : Nat := 5000
def maxVisitedProofTerms : Nat := 100000

def proofPathId (path : Array Nat) (depth : Nat) : String :=
  let pathText := if path.isEmpty then "root" else String.intercalate "." (path.map toString).toList
  s!"local@{pathText}:{depth}"

def pushUniqueNat (values : Array Nat) (value : Nat) : Array Nat :=
  if values.contains value then values else values.push value

def pushUniqueString (values : Array String) (value : String) : Array String :=
  if values.contains value then values else values.push value

def usedFreeVariable (expr : Expr) (id : FVarId) : Bool :=
  (expr.find? fun candidate => candidate.isFVar && candidate.fvarId! == id).isSome

def isRecursorName (env : Environment) (name : Name) : Bool :=
  match env.find? name with
  | some (.recInfo _) => true
  | _ => false

def isBranchingApplicationName (env : Environment) (name : Name) : Bool :=
  isRecursorName env name || name == `ite || name == `dite ||
    name == `Decidable.byCases || name == `Classical.byCases

def markProofExtractionIncomplete (builder : IO.Ref ProofStepBuilder) (reason : String) : IO Unit := do
  builder.modify fun state =>
    if state.complete then { state with complete := false, truncationReason := some reason } else state

def countVisitedProofTerm (builder : IO.Ref ProofStepBuilder) : IO Bool := do
  let state ← builder.get
  if state.visitedTerms >= maxVisitedProofTerms then
    markProofExtractionIncomplete builder s!"proof-term visit limit ({maxVisitedProofTerms}) reached"
    return false
  builder.modify fun current => { current with visitedTerms := current.visitedTerms + 1 }
  return true

def snapshotContextEntry (id : String) (userName : Name) (kind : String)
    (binderInfo : BinderInfo) (type : Expr) (value? : Option Expr := none) : MetaM ProofContextEntry := do
  return {
    id
    userName := toString userName
    kind
    binderInfo := binderInfoName binderInfo
    type := ← prettyPrintExpr type
    typeGraph := expressionGraph type
    value := ← value?.mapM prettyPrintExpr
  }

def addProofCandidate (builder : IO.Ref ProofStepBuilder) (expr proposition : Expr) (kind : String)
    (context : Array ProofContextEntry) (bindings : Array (FVarId × String × Option Nat))
    (path : Array Nat) (childSteps : Array Nat) (namedReference? : Option Name) : MetaM (Option Nat) := do
  let state ← builder.get
  if state.steps.size >= maxProofSteps then
    markProofExtractionIncomplete builder s!"candidate-step limit ({maxProofSteps}) reached"
    return none
  let mut prerequisites := childSteps
  let mut hypothesisReferences := #[]
  for (fvarId, localId, localStep?) in bindings do
    if usedFreeVariable expr fvarId then
      match localStep? with
      | some stepId => prerequisites := pushUniqueNat prerequisites stepId
      | none =>
          if context.any fun entry => entry.id == localId && entry.kind == "assumption" then
            hypothesisReferences := pushUniqueString hypothesisReferences localId
  let namedReferences := match namedReference? with
    | some name => #[toString name]
    | none => #[]
  let id := state.steps.size
  let step : ProofStep := {
    id
    kind
    proposition := ← prettyPrintExpr proposition
    propositionGraph := expressionGraph proposition
    context
    scope := context.map (·.id)
    proofTermPath := path
    prerequisiteSteps := prerequisites.qsort (· < ·)
    hypothesisReferences := hypothesisReferences.qsort (· < ·)
    namedReferences
  }
  builder.modify fun current => { current with steps := current.steps.push step }
  return some id

partial def walkProofTerm (builder : IO.Ref ProofStepBuilder) (expr : Expr)
    (context : Array ProofContextEntry) (bindings : Array (FVarId × String × Option Nat))
    (path : Array Nat) (forcedKind? : Option String := none) : MetaM (Array Nat) := do
  unless ← countVisitedProofTerm builder do return #[]
  match expr with
  | .lam binderName domain body binderInfo =>
      Meta.withLocalDecl binderName binderInfo domain fun fvar => do
        let localId := proofPathId path context.size
        let kind := if ← Meta.isProp domain then "assumption" else "variable"
        let entry ← snapshotContextEntry localId binderName kind binderInfo domain
        walkProofTerm builder (body.instantiate1 fvar) (context.push entry)
          (bindings.push (fvar.fvarId!, localId, none)) (path.push 0) forcedKind?
  | .letE binderName type value body nondep =>
      let localSteps ← walkProofTerm builder value context bindings (path.push 0) (some "localFact")
      let localStep? := localSteps.back?
      Meta.withLetDecl binderName type value (nondep := nondep) fun fvar => do
        let localId := proofPathId path context.size
        let kind := if ← Meta.isProp type then "localFact" else "definition"
        let entry ← snapshotContextEntry localId binderName kind BinderInfo.default type (some value)
        walkProofTerm builder (body.instantiate1 fvar) (context.push entry)
          (bindings.push (fvar.fvarId!, localId, localStep?)) (path.push 1) forcedKind?
  | .mdata _ body =>
      walkProofTerm builder body context bindings (path.push 0) forcedKind?
  | .proj _ _ body =>
      let childSteps ← walkProofTerm builder body context bindings (path.push 0)
      match forcedKind? with
      | none => return childSteps
      | some kind =>
          let proposition ← instantiateMVars (← Meta.inferType expr)
          if ← Meta.isProp proposition then
            return (← addProofCandidate builder expr proposition kind context bindings path
              childSteps none).map (#[·]) |>.getD #[]
          else return childSteps
  | .app .. =>
      let head := expr.getAppFn
      let args := expr.getAppArgs
      let headName? := match head with | .const name _ => some name | _ => none
      let env ← getEnv
      let isBranching := headName?.any (isBranchingApplicationName env)
      let mut childSteps := #[]
      for h : index in [:args.size] do
        let arg := args[index]
        let branchKind := if isBranching && arg.isLambda then some "branchConclusion" else none
        for childId in ← walkProofTerm builder arg context bindings (path.push index) branchKind do
          childSteps := pushUniqueNat childSteps childId
      let proposition ← instantiateMVars (← Meta.inferType expr)
      if ← Meta.isProp proposition then
        match forcedKind?, headName? with
        | some kind, _ =>
            return (← addProofCandidate builder expr proposition kind context bindings path childSteps headName?).map (#[·]) |>.getD #[]
        | none, some name =>
            return (← addProofCandidate builder expr proposition "namedApplication" context bindings path childSteps (some name)).map (#[·]) |>.getD #[]
        | none, none =>
            return (← addProofCandidate builder expr proposition "anonymousApplication" context bindings path childSteps none).map (#[·]) |>.getD #[]
      else
        return childSteps
  | .const name _ =>
      match forcedKind? with
      | none => return #[]
      | some kind =>
          let proposition ← instantiateMVars (← Meta.inferType expr)
          if ← Meta.isProp proposition then
            return (← addProofCandidate builder expr proposition kind context bindings path #[] (some name)).map (#[·]) |>.getD #[]
          else return #[]
  | .fvar .. | .bvar .. | .mvar .. | .sort .. | .forallE .. | .lit .. =>
      match forcedKind? with
      | none => return #[]
      | some kind =>
          let proposition ← instantiateMVars (← Meta.inferType expr)
          if ← Meta.isProp proposition then
            return (← addProofCandidate builder expr proposition kind context bindings path #[] none).map (#[·]) |>.getD #[]
          else return #[]

def snapshotNamedResult (name : Name) : MetaM NamedResultSnapshot := do
  let env ← getEnv
  let some info := env.find? name
    | throwError "unknown proof-step reference '{name}'"
  return {
    name := toString name
    kind := declarationKind info
    moduleName := moduleForDeclaration? env name
    type := ← prettyPrintExpr info.type
    typeGraph := expressionGraph info.type
  }

def extractProofSteps (value : Expr) : MetaM ProofStepExtraction := do
  let builder ← IO.mkRef ({} : ProofStepBuilder)
  let conclusionSteps ← walkProofTerm builder value #[] #[] #[] (some "conclusion")
  let conclusionStep := conclusionSteps.back?
  let state ← builder.get
  let mut referenced : NameSet := {}
  for step in state.steps do
    for name in step.namedReferences do
      referenced := referenced.insert (parseName name)
  let names := referenced.toArray.qsort fun left right => toString left < toString right
  let namedResults ← names.mapM snapshotNamedResult
  return {
    complete := state.complete
    truncationReason := state.truncationReason
    visitedTerms := state.visitedTerms
    conclusionStep
    steps := state.steps
    namedResults
  }

def snapshotDeclaration (name : Name) (includeProofSteps : Bool := false) : MetaM DeclarationSnapshot := do
  let env ← getEnv
  let some info := env.find? name
    | throwError "unknown declaration '{name}'"
  let value? := info.value? true
  let axioms ← Lean.collectAxioms name
  let docString ← Lean.findDocString? env name
  let sourceRange := (← Lean.findDeclarationRanges? name).map (·.range)
  let proofSteps ← if includeProofSteps then value?.mapM extractProofSteps else pure none
  return {
    name := toString name
    kind := declarationKind info
    moduleName := moduleForDeclaration? env name
    generationKind := ← generationKind? env name info
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
    proofSteps
  }

def snapshotSymbol (name : Name) : MetaM SymbolSnapshot := do
  let env ← getEnv
  let some info := env.find? name
    | throwError "unknown referenced declaration '{name}'"
  return {
    name := toString name
    kind := declarationKind info
    moduleName := moduleForDeclaration? env name
    generationKind := ← generationKind? env name info
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

def coreContext : Core.Context := {
  fileName := "<sieve-extractor>"
  fileMap := FileMap.ofString ""
}

def extract (env : Environment) (importedModules : Array Name) (targets : Array Name)
    (includeProofSteps : Bool := false) : IO ExtractionSnapshot := do
  let action : MetaM (Array DeclarationSnapshot × Array SymbolSnapshot) := do
    let declarations ← targets.mapM (snapshotDeclaration · includeProofSteps)
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
    importedModules := importedModules.map toString
    declarations
    symbols
  }

structure ExtractOptions where
  modules : Array Name := #[]
  targets : Array Name := #[]
  includeProofSteps : Bool := false

partial def parseExtractOptions (args : List String) (options : ExtractOptions := {}) : Except String ExtractOptions :=
  match args with
  | [] => pure options
  | "--proof-steps" :: rest =>
      parseExtractOptions rest { options with includeProofSteps := true }
  | "--module" :: moduleName :: rest =>
      parseExtractOptions rest { options with modules := options.modules.push (parseName moduleName) }
  | "--" :: targets =>
      pure { options with targets := targets.toArray.map parseName }
  | option :: _ =>
      throw s!"unknown or incomplete extractor option '{option}'"

unsafe def extractMain (args : List String) : IO Unit := do
  let options ← match parseExtractOptions args with
    | .ok options => pure options
    | .error message => throw (IO.userError message)
  if options.modules.isEmpty then
    throw (IO.userError "at least one --module is required")
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let imports := options.modules.map fun moduleName => ({ module := moduleName } : Import)
  let env ← importModules (loadExts := true) imports {}
  let targets := if options.targets.isEmpty then
    options.modules.foldl (fun names moduleName => names ++ declarationsInModule env moduleName) #[]
  else
    options.targets
  let snapshot ← extract env options.modules targets options.includeProofSteps
  IO.println (toJson snapshot).compress

end Sieve

unsafe def main := Sieve.extractMain
