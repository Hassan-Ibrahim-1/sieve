import type { Bootstrap, GraphResponse, ProofOutlineResponse, WitnessResponse } from "./types";
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
      includeTheorems: String(state.filters.includeTheorems),
      includeDefinitions: String(state.filters.includeDefinitions),
    });
    return request<GraphResponse>(`/api/ui/graph?${query}`, signal);
  },
  proofOutline(name: string, signal?: AbortSignal) {
    const query = new URLSearchParams({ name });
    return request<ProofOutlineResponse>(`/api/ui/proof-outline?${query}`, signal);
  },
  witnesses(source: string, target: string, limit: number, signal?: AbortSignal) {
    const query = new URLSearchParams({ source, target, limit: String(limit) });
    return request<WitnessResponse>(`/api/ui/witnesses?${query}`, signal);
  },
};
