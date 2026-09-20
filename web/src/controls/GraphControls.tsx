import type { Dispatch } from "react";
import type { Bootstrap } from "../api/types";
import type { Action, UiState } from "../app/state";
import { SearchControl } from "./SearchControl";

interface Props {
  state: UiState;
  dispatch: Dispatch<Action>;
  bootstrap?: Bootstrap;
  onSearchSelect: (id: string) => void;
}

export function GraphControls({ state, dispatch, bootstrap, onSearchSelect }: Props) {
  return (
    <aside className="sidebar" aria-label="Graph controls">
      <section>
        <h2>View</h2>
        <Check label="Emphasize most used" checked={state.mostUsed}
          onChange={(mostUsed) => dispatch({ type: "patch", value: { mostUsed } })} />
      </section>

      <SearchControl value={state.search} onChange={(search) => dispatch({ type: "patch", value: { search } })} onSelect={onSearchSelect} />

      <RangeField label="Maximum witness paths" value={state.witnessLimit} min={1} max={8} step={1}
        display={String(state.witnessLimit)} onChange={(witnessLimit) => dispatch({ type: "patch", value: { witnessLimit } })} />

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
