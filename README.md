# Sieve

Sieve extracts structural information from elaborated declarations in any
Lake-based Lean project so that formal proofs can be inspected with statistics
and visualizations. It runs the extractor with the target project's toolchain
and dependency search path; the project does not need to depend on Sieve or
Mathlib.

Build the Lean project first, then name one or more compiled modules to analyze:

```sh
lake build
/path/to/sieve --project /path/to/lean-project \
  --import MyProject.Basic --import MyProject.Advanced summary
```

`--project` defaults to the current directory. `--import` is repeatable and
required. With no declaration names, Sieve includes declarations defined
directly in every imported module. Imported dependencies remain available for
symbol metadata and targeted inspection, but are not automatically added to the
corpus. The existing `--module` option is an analysis filter and is distinct
from `--import`.

When developing Sieve itself, pass project options after Cargo's `--` separator:

```sh
cargo run -- --project ../my-lean-project --import MyProject.Basic summary
```

The Lean extractor obtains each declaration from the elaborated environment and
emits JSON. The Rust process deserializes and validates the snapshot and keeps it
in memory; no analysis database is used.

## Extracted data

For every corpus declaration, Sieve retains:

- declaration kind, module, source range, documentation, universe parameters,
  and internal/private/unsafe/partial flags;
- the pretty-printed type and transitive axioms;
- complete hash-consed expression DAGs for the type and value/proof;
- expression kinds, child edges, constants, binders, de Bruijn indices,
  projections, literals, and universe levels;
- direct statement and value dependencies, including occurrence counts;
- aggregate expression-shape statistics.

The snapshot also contains a symbol table for every directly referenced Lean
declaration. Symbol metadata includes its defining module, declaration kind,
and whether it is a class, instance, or projection. This allows dependency
statistics to distinguish mathematical declarations from common elaboration
infrastructure.

Rust validates the snapshot and builds a reusable `AnalysisCorpus` with
declaration and symbol indexes, typed forward and reverse dependency edges,
occurrence multiplicities, and cached structural fingerprints. Analysis
functions return serializable result values rather than printing directly.
The stored DAGs support repeated-subexpression analysis and structural
comparison without losing original occurrence counts. Raw and alpha-normalized
fingerprints only select candidates; Sieve verifies matches against the complete
structure to rule out hash collisions.

Tactic invocations and before/after goal states are not part of this compiled
snapshot. They require a separate source re-elaboration pipeline and should not
be inferred from the final proof term.

## Inspect proof steps

Extract occurrence-scoped intermediate claims from one elaborated proof:

```sh
FTC=Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
cargo run -- --import "$FTC" proof-steps \
  intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le
cargo run -- --import "$FTC" proof-steps \
  intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le \
  --step 28 --format json
```

Each candidate records its Lean-inferred proposition and structured expression,
full local context, proof-term path, direct prerequisite steps, used hypotheses,
and named results. Edges are oriented from prerequisite to the claim that uses
it. Selecting a step adds its direct and transitive prerequisites, direct
dependents, and paths to the final conclusion. Named-result statements are
included even when the declarations live outside the extracted module.

The extractor currently has conservative hard limits and reports
`complete: false` with a reason rather than silently returning partial evidence. The
[FTC evaluation reference](docs/ftc-proof-steps-reference.md) documents the
mathematical structure expected in the raw graph.

`Sieve/ProofStepFixtures.lean` supplies small proofs for reused facts,
identical branch conclusions under different assumptions, and the same
proposition proved by different arguments.

## Analyze

This repository retains Mathlib's Fundamental Theorem of Calculus module at
commit `dec5b2b780537b6eaf7f5e5f000c12f7387fb24d` as an evaluation corpus, not
as Sieve's runtime default. It contains 104 declarations, including 90 theorems
and generated or private declarations.

```sh
lake update
FTC=Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
cargo run -- --import "$FTC" summary
```

The experimental lens commands provide evidence-backed discovery and focused
inspection without assigning a single mathematical-importance score:

```sh
cargo run -- --import "$FTC" discover --lens influence --limit 20
cargo run -- --import "$FTC" discover --lens bridge --format json
cargo run -- --import "$FTC" inspect intervalIntegral.integral_deriv_eq_sub --lens neighbors
cargo run -- --import "$FTC" inspect \
  intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le \
  --lens proof --format json
cargo run -- --import "$FTC" inspect intervalIntegral.integral_deriv_eq_sub --lens trust
```

`discover` accepts `influence`, `bridge`, `neighbors`, or `all`; `inspect`
also accepts `proof` and `trust`. Influence and bridge use proof dependencies
by default, while statement neighbors use statement expressions. An explicit
`--layer both` produces separate statement and proof evidence rather than a
blended score. JSON reports include typed configuration, complete evidence,
canonical graph-region identifiers, and witness paths. Generated/private and
infrastructure declarations stay out of reported candidates by default, but
may occur as intermediate nodes in dependency paths.

Influence ranks by reachable dependents, direct dependents, and name. Bridge
ranks by articulation status, distinct directed graph-region pairs,
betweenness, and name. Neighbor reports use the existing combined structural
similarity and always expose dependency overlap, expression-kind, size, and
depth components. These are statement neighbors, not claims of equivalence,
generalization, or membership in a mathematical family.

Proof inspection runs normal full-corpus extraction plus targeted proof-step
extraction for the requested theorem. Its conservative outline retains the
conclusion, local facts, branch conclusions, and non-plumbing named
applications. The JSON report also retains the complete raw step graph, and
every condensed edge records the inclusive raw-step path it replaced. Trust
reports axioms, declaration flags, source/documentation availability, and
proof-extraction completeness without a safety score.

An intentionally repetitive generated-proof experiment is available without
changing the FTC evaluation corpus:

```sh
cargo run --example slop_duplicate_experiment
```

It adds three neutral-named declarations with the same theorem statement but
different proof plumbing. The report checks their exact/alpha-equivalence and
statement-neighbor evidence, then compares the reused declaration's influence
before and after augmentation. This demonstrates duplicate recognition and
reuse evidence; it does not label the group a mathematical family or assign a
mathematical-importance score.

Useful commands include:

```sh
FTC=Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
cargo run -- --import "$FTC" declaration intervalIntegral.integral_deriv_eq_sub
cargo run -- --import "$FTC" dependencies intervalIntegral.integral_deriv_eq_sub --layer proof
cargo run -- --import "$FTC" dependents intervalIntegral.integral_deriv_eq_sub --internal-only
cargo run -- --import "$FTC" path intervalIntegral.integral_deriv_eq_sub_uIoo intervalIntegral.integral_deriv_eq_sub
cargo run -- --import "$FTC" rank proof-occurrences --limit 20
cargo run -- --import "$FTC" compare intervalIntegral.integral_deriv_eq_sub intervalIntegral.integral_deriv_eq_sub' --layer both
cargo run -- --import "$FTC" duplicates --alpha
cargo run -- --import "$FTC" repeated-structures --layer proof --minimum-size 20 --minimum-support 2
cargo run -- --import "$FTC" modules
cargo run -- --import "$FTC" graph --internal-only
```

## Local API

Start the analysis server with:

```sh
FTC=Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
cargo run -- --import "$FTC" serve
```

After Sieve extracts and indexes the selected modules, open
`http://127.0.0.1:4173`. The built-in landing page links to the in-process Rust
analysis API. Declaration details, dependency links, structural comparisons,
and repeated fragments come from extracted Lean data rather than fixture data.
To use another port, add `--port 8080`.

Inspect a bounded prefix of an elaborated proof tree:

```sh
FTC=Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
cargo run -- --import "$FTC" declaration intervalIntegral.integral_deriv_eq_sub --tree-depth 4
```

Common filters are explicit and are included in every structured result:
`--layer statement|proof|both`, `--include-generated`,
`--include-infrastructure`, `--internal-only`, `--source-backed-only`,
`--kind`, and `--module`. Traversals also accept `--max-depth` and `--limit`.
Summary histogram boundaries can be changed with `--histogram-buckets N,N,...`.
Generated/private declarations and class/instance/projection infrastructure are
excluded by default but remain directly inspectable. Use the corresponding
include flags when their elaborated structure is relevant.

Use `--format json` for nested results. Flat deterministic CSV tables for
visualization are available through:

```sh
FTC=Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus
cargo run -- --import "$FTC" export nodes > nodes.csv
cargo run -- --import "$FTC" export edges --include-infrastructure > edges.csv
cargo run -- --import "$FTC" export modules > modules.csv
cargo run -- --import "$FTC" export metrics > metrics.csv
cargo run -- --import "$FTC" export duplicates > duplicates.csv
cargo run -- --import "$FTC" export repeated --minimum-size 20 > repeated.csv
```

The `graph` command reports degrees, weighted degrees, PageRank, unweighted
Brandes betweenness, connected and strongly connected components, articulation
points, bridges, and deterministic label-propagation communities. Its output
records the graph layer, filters, weight setting, algorithm, and parameters.

## Interpretation

These analyses are navigation aids, not measures of mathematical merit. A large
elaborated proof need not be mathematically complex, and a frequently used
declaration need not be fundamental. Direct proof-term dependencies do not
recover the author's thought process. Module boundaries are organizational
signals; type-class instances, projections, equality machinery, and generated
declarations can dominate raw counts. Structural similarity does not imply the
same mathematical argument.

The legacy command form with one or more fully-qualified declaration names is
retained for targeted extraction and inspection, but it still requires project
options.

## Test

```sh
lake build
cargo test
```

The full-module integration test is intentionally excluded from the normal
test run because loading and validating the entire compiled Mathlib module is
slow. Run it explicitly with:

```sh
cargo test extracts_an_analysis_ready_ftc_module -- --ignored
```
