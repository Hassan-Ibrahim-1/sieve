import type { ProofOutlineNode, ProofOutlineResponse } from "../api/types";
import { humanize } from "./proofGraphModel";

export function ProofInspector({ data, selected }: { data?: ProofOutlineResponse; selected?: number }) {
  const step = data?.outline.retainedNodes.find((node) => node.rawStepId === selected);
  return <aside className="inspector" data-open={Boolean(step)} aria-label="Proof step inspector">
    {!step && <div className="empty-inspector"><div className="selection-glyph">⌁</div><p>Select a proof step</p></div>}
    {step && <StepDetails step={step} conclusion={step.rawStepId === data?.outline.conclusionStep} />}
  </aside>;
}

function StepDetails({ step, conclusion }: { step: ProofOutlineNode; conclusion: boolean }) {
  return <>
    <div className="kind-row"><span>{conclusion ? "Conclusion" : humanize(step.kind)}</span><code>step #{step.rawStepId}</code></div>
    <pre className="proof-proposition"><code>{step.proposition}</code></pre>
    {step.namedReferences.length > 0 && <ReferenceList title="Named references" values={step.namedReferences} />}
    {step.hypothesisReferences.length > 0 && <ReferenceList title="Hypotheses" values={step.hypothesisReferences} />}
    {step.context.length > 0 && <section className="proof-context">
      <h2>Local context</h2>
      {step.context.map((entry) => <div className="context-entry" key={entry.id}>
        <strong>{entry.userName || entry.id}</strong><code>{entry.type}</code>
      </div>)}
    </section>}
  </>;
}

function ReferenceList({ title, values }: { title: string; values: string[] }) {
  return <section className="proof-references"><h2>{title}</h2>{values.map((value) => <code key={value}>{value}</code>)}</section>;
}
