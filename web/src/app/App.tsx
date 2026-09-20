import { useEffect, useMemo, useReducer, useState } from "react";
import { api } from "../api/client";
import type { Bootstrap, GraphEdge, GraphResponse, WitnessResponse } from "../api/types";
import { GraphControls } from "../controls/GraphControls";
import { GraphCanvas } from "../graph/GraphCanvas";
import { GraphHeader } from "../graph/GraphHeader";
import { Inspector } from "../inspector/Inspector";
import { useTheme } from "../theme/useTheme";
import { AppShell } from "./AppShell";
import { initialState, reducer, stateFromUrl, writeStateToUrl } from "./state";

export function App() {
  const [state, dispatch] = useReducer(reducer, undefined, () => typeof location === "undefined" ? initialState : stateFromUrl(location.search));
  const [bootstrap, setBootstrap] = useState<Bootstrap>();
  const [data, setData] = useState<GraphResponse>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(true);
  const [witnesses, setWitnesses] = useState<WitnessResponse>();
  const { theme, toggle } = useTheme();

  useEffect(() => { api.bootstrap().then(setBootstrap).catch((error) => setError(error.message)); }, []);
  useEffect(() => writeStateToUrl(state), [state]);
  useEffect(() => {
    const controller = new AbortController();
    setBusy(true); setError(undefined);
    api.graph(state, controller.signal).then(setData).catch((error) => { if (error.name !== "AbortError") setError(error.message); }).finally(() => setBusy(false));
    return () => controller.abort();
  }, [state.filters]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (witnesses) { setWitnesses(undefined); return; }
      dispatch({ type: "select", id: undefined });
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  });

  const selectedNode = useMemo(() => data?.nodes.find((node) => node.id === state.selected), [data, state.selected]);
  const selectedEdge = useMemo(() => data?.edges.find((edge) => edge.id === state.selectedEdge), [data, state.selectedEdge]);

  const showWitnesses = (edge: GraphEdge) => {
    setBusy(true);
    api.witnesses(edge.source, edge.target, state.witnessLimit)
      .then(setWitnesses).catch((error) => setError(error.message)).finally(() => setBusy(false));
  };

  const openEdge = (id: string) => {
    const edge = data?.edges.find((item) => item.id === id);
    if (!edge) return;
    if (edge.witnessAvailable) showWitnesses(edge);
  };

  const selectNode = (id?: string) => {
    dispatch({ type: "select", id });
  };

  return <AppShell theme={theme} onTheme={toggle}
    controls={<GraphControls state={state} dispatch={dispatch} bootstrap={bootstrap} />}
    inspector={<Inspector node={selectedNode} edge={selectedEdge} witnesses={witnesses} busy={busy}
      onWitnesses={showWitnesses} onCloseDetail={() => setWitnesses(undefined)} />}>
    {data && <GraphHeader data={data} />}
    <div className="graph-stage" data-loading={busy}>
      {data && <GraphCanvas data={data} mostUsed={state.mostUsed} selected={state.selected} selectedEdge={state.selectedEdge}
        theme={theme}
        onSelect={selectNode} onSelectEdge={(id) => dispatch({ type: "selectEdge", id })}
        onOpen={selectNode} onOpenEdge={openEdge} />}
      {busy && !data && <div className="loading-state"><span className="loading-ring" /></div>}
      {error && <div className="error-state"><span>!</span><p>{error}</p><button onClick={() => location.reload()}>Retry</button></div>}
      {!busy && !error && data?.nodes.length === 0 && <div className="empty-state">No statements match the current filters.</div>}
    </div>
  </AppShell>;
}
