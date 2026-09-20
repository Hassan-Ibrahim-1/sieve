import type { Bootstrap, Comparison, GraphResponse, ProofResponse, SearchResponse, WitnessResponse } from "./types";
import type { UiState } from "../app/state";

async function request<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(path, { signal, headers: { Accept: "application/json" } });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(payload.error ?? `Request failed (${response.status})`);
  return payload as T;
}

export const api = {
  bootstrap: (signal?: AbortSignal) => request<Bootstrap>("/api/ui/bootstrap", signal),
  graph(state: UiState, signal?: AbortSignal) {
    const query = new URLSearchParams({
      mode: state.mode === "proof" ? "similarity" : state.mode,
      level: state.level === "proof" ? "neighborhood" : state.level,
      metric: state.metric,
      limit: String(state.limit),
      threshold: String(state.threshold),
      depth: String(state.depth),
      includeTheorems: String(state.filters.includeTheorems),
      includeDefinitions: String(state.filters.includeDefinitions),
      includeTechnical: String(state.filters.includeTechnical),
    });
    if (state.scope) query.set("scope", state.scope);
    return request<GraphResponse>(`/api/ui/graph?${query}`, signal);
  },
  proof(declaration: string, detail = "outline", path?: string, signal?: AbortSignal) {
    const query = new URLSearchParams({ declaration, detail });
    if (path) query.set("path", path);
    return request<ProofResponse>(`/api/ui/proof?${query}`, signal);
  },
  compare(left: string, right: string, signal?: AbortSignal) {
    const query = new URLSearchParams({ left, right });
    return request<Comparison[]>(`/api/ui/compare?${query}`, signal);
  },
  witnesses(source: string, target: string, limit: number, signal?: AbortSignal) {
    const query = new URLSearchParams({ source, target, limit: String(limit) });
    return request<WitnessResponse>(`/api/ui/witnesses?${query}`, signal);
  },
  search(queryText: string, signal?: AbortSignal) {
    return request<SearchResponse>(`/api/ui/search?q=${encodeURIComponent(queryText)}&limit=12`, signal);
  },
};
