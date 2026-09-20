import type { Bootstrap, ProofOutlineResponse } from "../api/types";

export function ProofControls({ bootstrap, declaration, data, onDeclaration }: {
  bootstrap?: Bootstrap;
  declaration?: string;
  data?: ProofOutlineResponse;
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

    {data && <>
      <section className="proof-summary">
        <h2>Outline</h2>
        <div className="fact"><span>Retained steps</span><strong>{data.outline.retainedNodes.length}</strong></div>
        <div className="fact"><span>Raw steps</span><strong>{data.outline.rawSteps.length}</strong></div>
        <div className="fact"><span>Visited terms</span><strong>{data.outline.visitedTerms}</strong></div>
      </section>
      {!data.outline.complete && <div className="proof-warning">
        Partial extraction{data.outline.truncationReason ? `: ${data.outline.truncationReason}` : ""}
      </div>}
    </>}

    <div className="sidebar-spacer" />
    <p className="proof-help">Arrows run from prerequisite steps toward the proof conclusion.</p>
  </aside>;
}
