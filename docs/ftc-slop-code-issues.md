# FTC Slop and Sieve: Code-Issue Report

Date: 2026-09-19  
Environment: Lean `4.35.0-rc2`, Lake `5.0.0-src+11acb17`, Sieve schema `5`

## Executive summary

The discrete FTC development is logically sound and Sieve can extract and inspect
its main theorem when the declaration-bearing module `FtcSlop.Calculus` is used.
The proof-step result correctly exposes the endpoint arithmetic fact,
`FtcSlop.telescoping`, and the final rewrite.

The surrounding example and Sieve integration nevertheless have several issues:

| ID | Severity | Area | Finding |
| --- | --- | --- | --- |
| FTC-001 | High | Example executable | `FtcSlop.Main` defines no executable `main`; the linked binary runs an unrelated Lean exporter entry point. |
| FTC-002 | Medium | Lake configuration | The documented default `lake build` does not compile the root `FtcSlop` module, so Sieve cannot import it. |
| SIEVE-001 | High | Generated-declaration filtering | Lean-generated equation theorems are classified as authored declarations and survive the default generated filter. |
| SIEVE-002 | Medium | Empty-corpus handling | A successfully imported module with no declarations produces a successful, silent zero-result analysis. |
| DOC-001 | Medium | Example documentation | The example README does not provide a working Sieve command or explain which module contains the corpus. |
| TEST-001 | Medium | Integration coverage | Tests do not cover the example's build/import/run path or real Lean-generated declarations. |
| FTC-003 | Low | Lean source hygiene | `integral_succ` contains an unused simp argument and warns on every clean build. |

## Scope and expected behavior

`FtcSlop.lean` is an import-only wrapper:

```lean
import FtcSlop.Calculus
```

The declarations of interest are in `FtcSlop/Calculus.lean`. Sieve's documented
selection rule is to include declarations defined directly in each `--import`
module, not declarations from its imported dependencies. Consequently, a compiled
`FtcSlop` wrapper yielding zero corpus declarations is consistent with the current
design. The defects are that the normal build does not compile that wrapper, the
empty result is easy to mistake for successful analysis, and the example does not
tell users to import `FtcSlop.Calculus`.

## Detailed findings

### FTC-001: the executable has no `main` and runs unrelated code

Severity: **High**

`FtcSlop/Main.lean` defines `FtcSlop.square` and an anonymous compile-time
`example`, but no root-level executable entry point:

```lean
namespace FtcSlop

def square (n : Nat) : Int := Int.ofNat (n * n)

example : integral square 0 4 = square 4 - square 0 := by
  simpa using fundamental_theorem_of_calculus_at_zero square 4

end FtcSlop
```

The Lake file nevertheless declares this module as a `lean_exe` root. Direct
interpretation identifies the missing entry point:

```text
$ lake env lean --run FtcSlop/Main.lean
(interpreter) unknown declaration 'main'
```

More seriously, the native executable still links successfully. In the tested
toolchain, its link response includes `-lLeanExport`; because the project supplies
no intended entry point, running the resulting binary invokes exporter behavior:

```text
$ ./.lake/build/bin/ftc_slop
{"meta":{"exporter":{"name":"lean4export","version":"3.1.0"}, ...}}
$ echo $?
0
```

This is a false-success executable: it exits successfully but neither evaluates
the `square` example nor tests the FTC development.

Recommended correction:

1. If the fixture is analysis-only, remove `lean_exe ftc_slop` and make the Lean
   library the default build target.
2. If an executable is intentional, add a root-level `def main : IO Unit` that
   performs and prints a deterministic calculation, and add an assertion-based
   smoke test for its exact output.
3. Keep the theorem check as a named theorem if Sieve is expected to analyze it;
   an anonymous `example` is not part of the named declaration corpus.

Acceptance criterion: `lake env lean --run FtcSlop/Main.lean` and the native
binary both run the project's own entry point and produce the same expected output.

### FTC-002: the default build does not compile `FtcSlop.lean`

Severity: **Medium**

The Lake configuration marks only the executable as the default target:

```lean
lean_lib FtcSlop

@[default_target]
lean_exe ftc_slop where
  root := `FtcSlop.Main
```

`FtcSlop.Main` imports `FtcSlop.Calculus` directly. It does not import the root
`FtcSlop` wrapper. A clean `lake build` therefore creates:

```text
FtcSlop/Calculus.olean
FtcSlop/Main.olean
```

but not:

```text
FtcSlop.olean
```

The natural Sieve invocation then fails before analysis:

```text
$ cargo run -- --project examples/ftc-slop --import FtcSlop summary
Error: Lean extractor failed in project examples/ftc-slop:
object file '.../.lake/build/lib/lean/FtcSlop.olean' of module FtcSlop does not exist
```

Running `lake build FtcSlop` explicitly builds the missing wrapper. Once built,
Sieve reports zero declarations because the wrapper defines none. That zero result
is expected under the direct-module corpus rule; merely building the wrapper does
not make it the right analysis target.

Recommended correction:

- Document and test `--import FtcSlop.Calculus` as the canonical analysis command.
- If the root module is advertised as buildable, include the library in the
  default build as well, or remove the wrapper if it serves no purpose.
- Do not change Sieve to silently traverse all imports merely to compensate for
  this fixture. Such traversal would contradict the documented corpus boundary
  and could unexpectedly pull very large dependency trees into analyses.

Acceptance criterion: a clean checkout followed by the documented commands builds
every named analysis target and produces the intended seven authored declarations.

### SIEVE-001: equation theorems are not recognized as generated

Severity: **High**

Analyzing the correct module with default filters produces:

```text
$ cargo run -- --project examples/ftc-slop \
    --import FtcSlop.Calculus summary
corpus: 9 declarations, 7 theorems, 0 generated/private
```

The source defines two definitions and five theorems, for seven authored
declarations total. The extra theorems are generated equation lemmas:

- `FtcSlop.derivative.eq_1`
- `FtcSlop.integral.eq_1`

Sieve reports both with `internal=false`, `private=false`, and no source range.
They subsequently appear as ordinary candidates in discovery and neighbor
results. Adding `--source-backed-only` happens to remove them and yields the
expected `7 declarations, 5 theorems`, but source availability is not the same
concept as generated provenance and should not be the required workaround.

The extraction schema carries only `isInternal` and `isPrivate`; it has no
generated/provenance field. The extractor populates those flags using
`name.isInternal` and `isPrivateName`. Rust then defines "generated" everywhere
as the disjunction of those two unrelated properties:

```rust
declaration.is_internal || declaration.is_private
```

This affects:

- default declaration filtering;
- summary counts and histograms;
- discovery candidates and graph regions;
- trust evidence;
- HTTP API responses;
- CSV node and metric exports;
- the meaning of `--include-generated`.

Recommended correction:

1. Add explicit generated/provenance metadata to both Lean snapshot structures
   and their Rust equivalents. Do not infer it repeatedly in report layers.
2. Use Lean metadata such as `Lean.Meta.isEqnThm` for equation theorems instead
   of relying on `.eq_1` string matching.
3. Define the intended policy for other compiler-created declarations—recursors,
   match helpers, projections, no-confusion declarations, and auxiliary theorems—
   before naming the field simply `isGenerated`.
4. Centralize the policy in one method used by filters, metrics, lenses, server
   responses, and exporters.
5. Bump the extraction schema version if the serialized shape changes.

Acceptance criteria:

- Default analysis of `FtcSlop.Calculus` reports 7 declarations and 5 theorems.
- `--include-generated` reports 9 declarations and exposes both equation lemmas.
- Those lemmas are marked generated consistently in text, JSON, CSV, server, and
  trust output.
- A source-backed authored declaration is never marked generated merely because
  of its name, and a generated declaration does not become authored merely because
  it has a source range.

### SIEVE-002: empty corpora succeed without a diagnostic

Severity: **Medium**

After explicitly compiling `FtcSlop.lean`, Sieve accepts the module and prints a
normal-looking success report:

```text
Sieve schema 5 — Lean 4.35.0-rc2 — FtcSlop
corpus: 0 declarations, 0 theorems, 0 generated/private
expressions: statement 0 occurrences/0 unique nodes; value 0 occurrences/0 unique nodes
dependencies: 0 edges, 0 occurrences, 0 internal edges
```

The extractor's `declarationsInModule` function correctly selects declarations
whose defining module matches the requested module. Snapshot validation requires
at least one imported module but deliberately permits an empty declaration list.
No later command-level check warns that the selected corpus is empty.

An empty module is valid, so this should not necessarily be a hard error. For
interactive analysis, however, silent success hides wrong-module selections and
build/configuration mistakes.

Recommended correction:

- Emit a concise warning to stderr for corpus-oriented commands when all imported
  modules contribute zero direct declarations.
- Include the imported module names and restate the direct-definition rule.
- Preserve a zero exit status unless the product explicitly chooses a strict mode.
- Consider a distinct warning when declarations exist but all are removed by the
  active filters.

Acceptance criterion: importing the compiled wrapper succeeds but visibly warns
that `FtcSlop` contributes no direct declarations.

### DOC-001: the example does not document a working Sieve workflow

Severity: **Medium**

The example README describes itself as a project "for exercising Sieve" but ends
with only:

```sh
lake build
```

It does not state that:

- `FtcSlop.Calculus` is the declaration-bearing module;
- `FtcSlop` is an import-only wrapper;
- the default build does not compile that wrapper;
- the recommended command is, from the repository root:

  ```sh
  cargo run -- --project examples/ftc-slop \
    --import FtcSlop.Calculus summary
  ```

It also does not show proof inspection, which is the fixture's strongest useful
behavior:

```sh
cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus proof-steps \
  FtcSlop.fundamental_theorem_of_calculus
```

Recommended correction: add exact copy/paste build, summary, discovery, and
proof-step commands, including expected declaration counts after SIEVE-001 is
fixed.

### TEST-001: integration tests miss all observed failures

Severity: **Medium**

The active test suite passed (`29 passed`, one slow Mathlib test ignored), but it
does not exercise this example end to end. Generated-node tests construct
synthetic snapshots by setting `is_internal = generated`, which encodes the same
incorrect assumption as production code and cannot catch public equation lemmas.

Recommended additions:

1. A clean-build test proving which `.olean` targets the example documentation
   requires.
2. A process test that runs the executable and checks exact output.
3. A real Lean extraction test for `FtcSlop.Calculus` asserting authored/default
   and include-generated declaration sets separately.
4. An empty-module diagnostic test using `FtcSlop` or a dedicated fixture.
5. Cross-output assertions for text, JSON, CSV, trust, and server generated flags.

The small, dependency-free FTC project is a better default integration fixture
than relying only on the much larger Mathlib FTC module.

### FTC-003: unused simp argument

Severity: **Low**

Every clean build emits:

```text
warning: FtcSlop/Calculus.lean:28:54: This simp argument is unused:
  Int.add_assoc
```

The current proof is:

```lean
simp [integral, List.range_succ, List.foldl_append, Int.add_assoc]
```

Recommended correction: remove `Int.add_assoc` as Lean's linter suggests and
retain a clean build. This does not affect theorem correctness.

## Primary code locations

| Finding | Code locations |
| --- | --- |
| FTC-001 | [`FtcSlop/Main.lean`](../examples/ftc-slop/FtcSlop/Main.lean), [`lakefile.lean`](../examples/ftc-slop/lakefile.lean) |
| FTC-002 | [`FtcSlop.lean`](../examples/ftc-slop/FtcSlop.lean), [`lakefile.lean`](../examples/ftc-slop/lakefile.lean) |
| SIEVE-001 | [`Sieve/Extract.lean`](../Sieve/Extract.lean), [`src/model.rs`](../src/model.rs), [`src/analysis/filters.rs`](../src/analysis/filters.rs), [`src/analysis/metrics.rs`](../src/analysis/metrics.rs), [`src/analysis/lenses.rs`](../src/analysis/lenses.rs), [`src/server.rs`](../src/server.rs), [`src/report/csv.rs`](../src/report/csv.rs) |
| SIEVE-002 | [`Sieve/Extract.lean`](../Sieve/Extract.lean), [`src/validation.rs`](../src/validation.rs), [`src/main.rs`](../src/main.rs) |
| DOC-001 | [`examples/ftc-slop/README.md`](../examples/ftc-slop/README.md), [`README.md`](../README.md) |
| TEST-001 | [`src/main.rs`](../src/main.rs), [`src/analysis/corpus.rs`](../src/analysis/corpus.rs) |
| FTC-003 | [`FtcSlop/Calculus.lean`](../examples/ftc-slop/FtcSlop/Calculus.lean) |

## Verified correct behavior

The following behavior was confirmed and should be preserved:

- `lake build` elaborates the discrete FTC proof successfully.
- `FtcSlop.Calculus` extraction succeeds under the target project's toolchain.
- With `--source-backed-only`, the corpus contains exactly two definitions and
  five authored theorems.
- The dependency chain is coherent:
  `integral_succ` feeds `telescoping`, which feeds
  `fundamental_theorem_of_calculus`, which feeds the zero-based corollary.
- Proof-step extraction for `fundamental_theorem_of_calculus` is complete and
  records the endpoint fact from `Nat.add_sub_of_le`, use of `telescoping`, and
  the final equality rewrite.
- The active Rust test suite passes.

## Recommended implementation order

1. Fix or remove the invalid executable entry point (FTC-001).
2. Add real generated-declaration provenance and update every consumer
   (SIEVE-001), with the integration test written first.
3. Make the fixture's documented build and Sieve commands exact and reproducible
   (FTC-002 and DOC-001).
4. Add an empty-corpus warning (SIEVE-002).
5. Remove the linter warning and complete the end-to-end regression matrix
   (FTC-003 and TEST-001).

## Reproduction command matrix

Run from the repository root unless otherwise noted:

```sh
# Current default build
(cd examples/ftc-slop && lake build)

# Fails on a clean build because FtcSlop.olean is absent
cargo run -- --project examples/ftc-slop --import FtcSlop summary

# Working analysis target, but currently includes two generated equation lemmas
cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus summary

# Temporary workaround showing the authored corpus count
cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus --source-backed-only summary

# Correct proof-step extraction
cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus proof-steps \
  FtcSlop.fundamental_theorem_of_calculus

# Exposes the missing interpreter entry point
(cd examples/ftc-slop && lake env lean --run FtcSlop/Main.lean)

# Current native executable runs unrelated exporter behavior
(cd examples/ftc-slop && ./.lake/build/bin/ftc_slop)

# Existing automated tests
cargo test
```
