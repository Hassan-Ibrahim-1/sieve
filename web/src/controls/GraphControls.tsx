import type { Dispatch } from "react";
import type { Bootstrap, GraphMode, Metric } from "../api/types";
import type { Action, UiState } from "../app/state";
import { SearchControl } from "./SearchControl";

const modes: Array<{ id: GraphMode; label: string }> = [
  { id: "similarity", label: "Similar statements" },
  { id: "influence", label: "Most used" },
  { id: "connections", label: "Connections" },
  { id: "proof", label: "Proof steps" },
];

const metrics: Array<{ id: Metric; label: string }> = [
  { id: "recommended", label: "Recommended" },
  { id: "familySize", label: "Family size" },
  { id: "directDependents", label: "Direct dependents" },
  { id: "reachableDependents", label: "Reachable dependents" },
  { id: "bridgeEvidence", label: "Bridge evidence" },
  { id: "proofSize", label: "Proof size" },
];

interface Props {
  state: UiState;
  dispatch: Dispatch<Action>;
  bootstrap?: Bootstrap;
  onSearchSelect: (id: string) => void;
}

export function GraphControls({ state, dispatch, bootstrap, onSearchSelect }: Props) {
  const visibleModes = modes.filter((mode) => mode.id !== "proof" || state.proofDeclaration);
  const visibleMetrics = metrics.filter((metric) =>
    metric.id !== "proofSize" || state.mode === "proof" || state.level === "neighborhood",
  );
  return (
    <aside className="sidebar" aria-label="Graph controls">
      <section>
        <label className="field-label" htmlFor="graph-mode">Graph mode</label>
        <select id="graph-mode" value={state.mode}
          onChange={(event) => dispatch({ type: "mode", mode: event.target.value as GraphMode })}>
          {visibleModes.map((mode) => <option key={mode.id} value={mode.id}>{mode.label}</option>)}
        </select>
      </section>

      <SearchControl value={state.search} onChange={(search) => dispatch({ type: "patch", value: { search } })} onSelect={onSearchSelect} />

      <section>
        <label className="field-label" htmlFor="metric">Node size</label>
        <select id="metric" value={state.metric} onChange={(event) => dispatch({ type: "patch", value: { metric: event.target.value as Metric } })}>
          {visibleMetrics.map((metric) => <option key={metric.id} value={metric.id}>{metric.label}</option>)}
        </select>
      </section>

      {state.mode === "similarity" && (
        <RangeField label="Similarity threshold" value={state.threshold} min={0.55} max={0.95} step={0.05}
          display={state.threshold.toFixed(2)} onChange={(threshold) => dispatch({ type: "patch", value: { threshold } })} />
      )}
      {state.mode === "influence" && (
        <RangeField label="Neighborhood depth" value={state.depth} min={1} max={5} step={1}
          display={String(state.depth)} onChange={(depth) => dispatch({ type: "patch", value: { depth } })} />
      )}
      {state.mode === "connections" && (
        <RangeField label="Maximum witness paths" value={state.witnessLimit} min={1} max={8} step={1}
          display={String(state.witnessLimit)} onChange={(witnessLimit) => dispatch({ type: "patch", value: { witnessLimit } })} />
      )}
      {state.mode === "proof" && (
        <section>
          <span className="field-label">Proof detail</span>
          <div className="segmented">
            {(["outline", "raw"] as const).map((detail) => (
              <button key={detail} data-active={state.proofDetail === detail}
                onClick={() => dispatch({ type: "patch", value: { proofDetail: detail } })}>{detail}</button>
            ))}
          </div>
        </section>
      )}

      <RangeField label="Visible nodes" value={state.limit} min={20} max={160} step={10}
        display={String(state.limit)} onChange={(limit) => dispatch({ type: "patch", value: { limit } })} />

      <section>
        <h2>Declarations</h2>
        <Check label="Theorems" checked={state.filters.includeTheorems}
          onChange={(includeTheorems) => dispatch({ type: "patch", value: { filters: { ...state.filters, includeTheorems } } })} />
        <Check label="Definitions" checked={state.filters.includeDefinitions}
          onChange={(includeDefinitions) => dispatch({ type: "patch", value: { filters: { ...state.filters, includeDefinitions } } })} />
        <Check label="Technical declarations" checked={state.filters.includeTechnical}
          onChange={(includeTechnical) => dispatch({ type: "patch", value: { filters: { ...state.filters, includeTechnical } } })} />
      </section>

      <div className="sidebar-spacer" />
      {bootstrap && <div className="corpus-fingerprint" title={bootstrap.corpusFingerprint}>{bootstrap.declarationCount.toLocaleString()} declarations</div>}
      <button className="reset-button" onClick={() => dispatch({ type: "reset" })}>Reset graph</button>
    </aside>
  );
}

function RangeField({ label, value, min, max, step, display, onChange }: {
  label: string; value: number; min: number; max: number; step: number; display: string; onChange: (value: number) => void;
}) {
  return <section>
    <label className="range-label"><span>{label}</span><output>{display}</output></label>
    <input type="range" aria-label={label} value={value} min={min} max={max} step={step}
      onChange={(event) => onChange(Number(event.target.value))} />
  </section>;
}

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return <label className="check-row"><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span>{label}</span></label>;
}
