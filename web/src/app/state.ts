import type { UiFilters } from "../api/types";

export interface UiState {
  view: "sieve" | "proof";
  proofDeclaration?: string;
  mostUsed: boolean;
  witnessLimit: number;
  filters: UiFilters;
  selected?: string;
  selectedEdge?: string;
}

export type Action =
  | { type: "patch"; value: Partial<UiState> }
  | { type: "select"; id?: string }
  | { type: "selectEdge"; id?: string }
  | { type: "reset" };

export const initialState: UiState = {
  view: "sieve",
  mostUsed: false,
  witnessLimit: 3,
  filters: { includeTheorems: true, includeDefinitions: true },
};

export function reducer(state: UiState, action: Action): UiState {
  switch (action.type) {
    case "patch": return { ...state, ...action.value };
    case "select": return { ...state, selected: action.id, selectedEdge: undefined };
    case "selectEdge": return { ...state, selectedEdge: action.id, selected: undefined };
    case "reset": return initialState;
  }
}

export function stateFromUrl(search: string): UiState {
  const params = new URLSearchParams(search);
  return {
    ...initialState,
    view: params.get("view") === "proof" ? "proof" : "sieve",
    proofDeclaration: params.get("proof") || undefined,
    mostUsed: booleanParam(params, "mostUsed", false),
    witnessLimit: boundedNumber(params.get("witnessLimit"), 1, 8, initialState.witnessLimit),
    selected: params.get("selected") || undefined,
    selectedEdge: params.get("selectedEdge") || undefined,
    filters: {
      includeTheorems: booleanParam(params, "theorems", initialState.filters.includeTheorems),
      includeDefinitions: booleanParam(params, "definitions", initialState.filters.includeDefinitions),
    },
  };
}

function boundedNumber(value: string | null, minimum: number, maximum: number, fallback: number) {
  const parsed = value === null ? NaN : Number(value);
  return Number.isFinite(parsed) ? Math.min(maximum, Math.max(minimum, parsed)) : fallback;
}

function booleanParam(params: URLSearchParams, key: string, fallback: boolean) {
  const value = params.get(key);
  if (value === "1" || value === "true") return true;
  if (value === "0" || value === "false") return false;
  return fallback;
}

export function writeStateToUrl(state: UiState) {
  const params = new URLSearchParams();
  if (state.view === "proof") params.set("view", "proof");
  if (state.proofDeclaration) params.set("proof", state.proofDeclaration);
  params.set("mostUsed", state.mostUsed ? "1" : "0");
  params.set("witnessLimit", String(state.witnessLimit));
  params.set("theorems", state.filters.includeTheorems ? "1" : "0");
  params.set("definitions", state.filters.includeDefinitions ? "1" : "0");
  if (state.selected) params.set("selected", state.selected);
  if (state.selectedEdge) params.set("selectedEdge", state.selectedEdge);
  history.replaceState(null, "", `${location.pathname}?${params}`);
}
