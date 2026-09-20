import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { api } from "../api/client";
import type { Bootstrap, Breadcrumb, Comparison, GraphEdge, GraphNode, GraphResponse, WitnessResponse } from "../api/types";
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
    const load = state.mode === "proof" && state.proofDeclaration
      ? api.proof(state.proofDeclaration.replace(/^decl:/, ""), state.proofDetail, state.proofPath, controller.signal).then((proof) => proof.graph)
      : api.graph(state, controller.signal);
    load.then(setData).catch((error) => { if (error.name !== "AbortError") setError(error.message); }).finally(() => setBusy(false));
    return () => controller.abort();
  }, [state.mode, state.level, state.scope, state.metric, state.limit, state.threshold, state.depth, state.filters, state.proofDeclaration, state.proofDetail, state.proofPath]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Enter" && state.selected) {
        openNode(state.selected);
        return;
      }
      if (event.key !== "Escape") return;
      if (comparison || witnesses) { setComparison(undefined); setWitnesses(undefined); return; }
      if (!data || data.scope.breadcrumbs.length < 2) return;
      const crumb = data.scope.breadcrumbs.at(-2)!;
      navigate(crumb);
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  });

  const selectedNode = useMemo(() => data?.nodes.find((node) => node.id === state.selected), [data, state.selected]);
  const selectedEdge = useMemo(() => data?.edges.find((edge) => edge.id === state.selectedEdge), [data, state.selectedEdge]);

  const navigate = useCallback((crumb: Breadcrumb) => {
    if (crumb.level === "proof") return;
    dispatch({ type: "patch", value: { mode: state.mode === "proof" ? "similarity" : state.mode, proofDeclaration: undefined, proofPath: undefined } });
    dispatch({ type: "navigate", level: crumb.level, scope: crumb.id });
  }, [state.mode]);

  const openNode = useCallback((id: string) => {
    const node = data?.nodes.find((item) => item.id === id);
    if (!node) return;
    if (node.nodeKind === "family") dispatch({ type: "navigate", level: "family", scope: node.id });
    else if (node.nodeKind === "declaration") dispatch({ type: "navigate", level: "neighborhood", scope: node.id });
  }, [data]);

  const primaryAction = (node: GraphNode) => {
    if (node.nodeKind === "family") { openNode(node.id); return; }
    if (node.nodeKind === "declaration") {
      dispatch({ type: "pin", id: node.id });
      if (state.level !== "neighborhood") openNode(node.id);
    }
  };

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
    if (state.mode === "proof" && edge.kind === "condensedProofPath" && (edge.collapsedStepCount ?? 0) > 0) {
      dispatch({ type: "patch", value: { proofDetail: "raw", proofPath: edge.id } });
    } else if (edge.witnessAvailable) showWitnesses(edge);
  };

  const searchSelect = (id: string) => {
    dispatch({ type: "patch", value: { mode: "similarity", proofDeclaration: undefined } });
    dispatch({ type: "navigate", level: "neighborhood", scope: id });
  };

  const selectNode = (id?: string) => {
    dispatch({ type: "select", id });
    const node = data?.nodes.find((item) => item.id === id);
    if (node?.nodeKind === "declaration" && node.actions.includes("proof")) {
      dispatch({ type: "patch", value: { proofDeclaration: node.id } });
    }
  };

  return <AppShell theme={theme} onTheme={toggle}
    controls={<GraphControls state={state} dispatch={dispatch} bootstrap={bootstrap} onSearchSelect={searchSelect} />}
    inspector={<Inspector mode={state.mode} node={selectedNode} edge={selectedEdge} pins={state.pins} comparison={comparison} witnesses={witnesses} busy={busy}
      onPin={(id) => dispatch({ type: "pin", id })} onPrimary={primaryAction} onCompare={comparePins} onWitnesses={showWitnesses}
      onOpenEdge={(edge) => openEdge(edge.id)}
      onCloseDetail={() => { setComparison(undefined); setWitnesses(undefined); }} />}>
    {data && <GraphHeader data={data} onNavigate={navigate} />}
    <div className="graph-stage" data-loading={busy}>
      {data && <GraphCanvas data={data} metric={state.metric} selected={state.selected} selectedEdge={state.selectedEdge} pins={state.pins} search={state.search}
        theme={theme}
        layout={state.mode === "similarity" || state.mode === "connections" ? "force" : "layered"}
        onSelect={selectNode} onSelectEdge={(id) => dispatch({ type: "selectEdge", id })}
        onOpen={openNode} onOpenEdge={openEdge} />}
      {busy && !data && <div className="loading-state"><span className="loading-ring" /></div>}
      {error && <div className="error-state"><span>!</span><p>{error}</p><button onClick={() => location.reload()}>Retry</button></div>}
      {!busy && !error && data?.nodes.length === 0 && <div className="empty-state">No statements match the current filters.</div>}
    </div>
  </AppShell>;
}
