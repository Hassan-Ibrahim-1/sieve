import { useEffect, useMemo, useReducer, useState } from "react";
import { api } from "../api/client";
import type { Bootstrap, Comparison, GraphEdge, GraphResponse, WitnessResponse } from "../api/types";
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
  const [comparison, setComparison] = useState<Comparison[]>();
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
      if (comparison || witnesses) { setComparison(undefined); setWitnesses(undefined); return; }
      dispatch({ type: "select", id: undefined });
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  });

  const selectedNode = useMemo(() => data?.nodes.find((node) => node.id === state.selected), [data, state.selected]);
  const selectedEdge = useMemo(() => data?.edges.find((edge) => edge.id === state.selectedEdge), [data, state.selectedEdge]);

  const comparePins = () => {
    if (state.pins.length !== 2) return;
    setBusy(true);
    api.compare(...state.pins.map((id) => id.replace(/^decl:/, "")) as [string, string])
      .then(setComparison).catch((error) => setError(error.message)).finally(() => setBusy(false));
  };

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

  const searchSelect = (id: string) => {
    dispatch({ type: "select", id });
  };

  const selectNode = (id?: string) => {
    dispatch({ type: "select", id });
  };

  return <AppShell theme={theme} onTheme={toggle}
    controls={<GraphControls state={state} dispatch={dispatch} bootstrap={bootstrap} onSearchSelect={searchSelect} />}
    inspector={<Inspector node={selectedNode} edge={selectedEdge} pins={state.pins} comparison={comparison} witnesses={witnesses} busy={busy}
      onPin={(id) => dispatch({ type: "pin", id })} onCompare={comparePins} onWitnesses={showWitnesses}
      onCloseDetail={() => { setComparison(undefined); setWitnesses(undefined); }} />}>
    {data && <GraphHeader data={data} />}
    <div className="graph-stage" data-loading={busy}>
      {data && <GraphCanvas data={data} mostUsed={state.mostUsed} selected={state.selected} selectedEdge={state.selectedEdge} pins={state.pins} search={state.search}
        theme={theme}
        onSelect={selectNode} onSelectEdge={(id) => dispatch({ type: "selectEdge", id })}
        onOpen={selectNode} onOpenEdge={openEdge} />}
      {busy && !data && <div className="loading-state"><span className="loading-ring" /></div>}
      {error && <div className="error-state"><span>!</span><p>{error}</p><button onClick={() => location.reload()}>Retry</button></div>}
      {!busy && !error && data?.nodes.length === 0 && <div className="empty-state">No statements match the current filters.</div>}
    </div>
  </AppShell>;
}
