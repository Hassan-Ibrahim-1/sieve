import type { Dispatch } from "react";
import type { Bootstrap, ProofOutlineResponse } from "../api/types";
import type { Action, UiState } from "../app/state";

export function ProofControls({ bootstrap, declaration, data, state, dispatch, onDeclaration }: {
  bootstrap?: Bootstrap;
  declaration?: string;
  data?: ProofOutlineResponse;
  state: UiState;
  dispatch: Dispatch<Action>;
  onDeclaration: (name: string) => void;
}) {
  const declarations = bootstrap?.proofDeclarations ?? [];
  return <aside className="sidebar" aria-label="Proof controls">
    <section>
      <h2>Theorem</h2>
      <select aria-label="Theorem proof" value={declaration ?? ""}
        disabled={declarations.length === 0}
        onChange={(event) => onDeclaration(event.target.value)}>
        {declarations.length === 0 && <option value="">No extracted proofs</option>}
        {declarations.map((item) => <option key={item.name} value={item.name}>{item.name}</option>)}
      </select>
    </section>

    <section>
      <h2>View</h2>
      <label className="check-row"><input type="checkbox" checked={state.showLabels}
        onChange={(event) => dispatch({ type: "patch", value: { showLabels: event.target.checked } })} />
        <span>Show labels</span>
      </label>
    </section>

    {data && <>
      <section className="proof-summary">
        <h2>Outline</h2>
        <div className="fact"><span>Shown steps</span><strong>{data.outline.retainedNodes.length}</strong></div>
        {data.outline.candidateNodeCount > data.outline.retainedNodes.length &&
          <div className="fact"><span>Outline candidates</span><strong>{data.outline.candidateNodeCount}</strong></div>}
        <div className="fact"><span>Raw steps</span><strong>{data.outline.rawStepCount}</strong></div>
        <div className="fact"><span>Visited terms</span><strong>{data.outline.visitedTerms}</strong></div>
      </section>
      {!data.outline.complete && <div className="proof-warning">
        Partial extraction{data.outline.truncationReason ? `: ${data.outline.truncationReason}` : ""}
      </div>}
      {data.outline.retainedNodes.length >= 120 && <p className="proof-help">
        Large outline: opens near the conclusion. Pan to follow branches or zoom out for the full proof.
      </p>}
    </>}

    <div className="sidebar-spacer" />
    <p className="proof-help">Arrows run from prerequisite steps toward the proof conclusion.</p>
  </aside>;
}
