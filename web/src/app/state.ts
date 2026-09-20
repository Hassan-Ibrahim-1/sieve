import type { GraphLevel, GraphMode, Metric, UiFilters } from "../api/types";

export interface UiState {
  mode: GraphMode;
  level: GraphLevel;
  scope?: string;
  metric: Metric;
  threshold: number;
  depth: number;
  witnessLimit: number;
  proofDetail: "outline" | "raw";
  limit: number;
  filters: UiFilters;
  selected?: string;
  selectedEdge?: string;
  pins: string[];
  search: string;
  proofDeclaration?: string;
  proofPath?: string;
}

export type Action =
  | { type: "patch"; value: Partial<UiState> }
  | { type: "mode"; mode: GraphMode }
  | { type: "navigate"; level: GraphLevel; scope?: string }
  | { type: "select"; id?: string }
  | { type: "selectEdge"; id?: string }
  | { type: "pin"; id: string }
  | { type: "reset" };

export const initialState: UiState = {
  mode: "similarity",
  level: "corpus",
  metric: "recommended",
  threshold: 0.65,
  depth: 2,
  witnessLimit: 3,
  proofDetail: "outline",
  limit: 80,
  filters: { includeTheorems: true, includeDefinitions: true, includeTechnical: false },
  pins: [],
  search: "",
};

export function reducer(state: UiState, action: Action): UiState {
  switch (action.type) {
    case "patch": return { ...state, ...action.value };
    case "mode":
      return {
        ...state,
        mode: action.mode,
        level: action.mode === "proof" ? "proof" : state.mode === "proof" ? "neighborhood" : state.level,
        scope: action.mode !== "proof" && state.mode === "proof" ? state.proofDeclaration : state.scope,
        metric: "recommended",
        selected: undefined,
        selectedEdge: undefined,
      };
    case "navigate":
      return { ...state, level: action.level, scope: action.scope, selected: undefined, selectedEdge: undefined };
    case "select": return { ...state, selected: action.id, selectedEdge: undefined };
    case "selectEdge": return { ...state, selectedEdge: action.id, selected: undefined };
    case "pin": {
      const pins = state.pins.includes(action.id)
        ? state.pins.filter((id) => id !== action.id)
        : [...state.pins.slice(-1), action.id];
      return { ...state, pins };
    }
    case "reset":
      return { ...initialState, mode: state.mode, level: state.level, scope: state.scope, proofDeclaration: state.proofDeclaration };
  }
}

const validModes = new Set<GraphMode>(["similarity", "influence", "connections", "proof"]);
const validLevels = new Set<GraphLevel>(["corpus", "family", "neighborhood", "proof"]);
const validMetrics = new Set<Metric>([
  "recommended",
  "familySize",
  "directDependents",
  "reachableDependents",
  "bridgeEvidence",
  "proofSize",
]);

export function stateFromUrl(search: string): UiState {
  const params = new URLSearchParams(search);
  const mode = params.get("mode") as GraphMode;
  const level = params.get("level") as GraphLevel;
  const metric = params.get("metric") as Metric;
  return {
    ...initialState,
    mode: validModes.has(mode) ? mode : initialState.mode,
    level: validLevels.has(level) ? level : initialState.level,
    scope: params.get("scope") || undefined,
    metric: validMetrics.has(metric) ? metric : initialState.metric,
    threshold: boundedNumber(params.get("threshold"), 0.55, 1, initialState.threshold),
    depth: boundedNumber(params.get("depth"), 1, 5, initialState.depth),
    witnessLimit: boundedNumber(params.get("witnessLimit"), 1, 8, initialState.witnessLimit),
    limit: boundedNumber(params.get("limit"), 20, 160, initialState.limit),
    proofDetail: params.get("proofDetail") === "raw" ? "raw" : "outline",
    proofDeclaration: params.get("proof") || undefined,
    proofPath: params.get("proofPath") || undefined,
    selected: params.get("selected") || undefined,
    selectedEdge: params.get("selectedEdge") || undefined,
    pins: params.getAll("pin").slice(-2),
    filters: {
      includeTheorems: booleanParam(params, "theorems", initialState.filters.includeTheorems),
      includeDefinitions: booleanParam(params, "definitions", initialState.filters.includeDefinitions),
      includeTechnical: booleanParam(params, "technical", initialState.filters.includeTechnical),
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
  params.set("mode", state.mode);
  params.set("level", state.level);
  params.set("metric", state.metric);
  params.set("limit", String(state.limit));
  params.set("theorems", state.filters.includeTheorems ? "1" : "0");
  params.set("definitions", state.filters.includeDefinitions ? "1" : "0");
  params.set("technical", state.filters.includeTechnical ? "1" : "0");
  if (state.scope) params.set("scope", state.scope);
  if (state.selected) params.set("selected", state.selected);
  if (state.selectedEdge) params.set("selectedEdge", state.selectedEdge);
  for (const pin of state.pins) params.append("pin", pin);
  if (state.mode === "similarity") params.set("threshold", String(state.threshold));
  if (state.mode === "influence") params.set("depth", String(state.depth));
  if (state.mode === "connections") params.set("witnessLimit", String(state.witnessLimit));
  if (state.mode === "proof" && state.proofDeclaration) {
    params.set("proof", state.proofDeclaration);
    params.set("proofDetail", state.proofDetail);
    if (state.proofPath) params.set("proofPath", state.proofPath);
  }
  history.replaceState(null, "", `${location.pathname}?${params}`);
}
