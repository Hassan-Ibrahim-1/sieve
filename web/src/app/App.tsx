import { useEffect, useMemo, useReducer, useState } from "react";
import { api } from "../api/client";
import type { Bootstrap, GraphEdge, GraphResponse, ProofOutlineResponse, WitnessResponse } from "../api/types";
import { GraphControls } from "../controls/GraphControls";
import { GraphCanvas } from "../graph/GraphCanvas";
import { GraphHeader } from "../graph/GraphHeader";
import { Inspector } from "../inspector/Inspector";
import { ProofControls } from "../proof/ProofControls";
import { ProofGraph } from "../proof/ProofGraph";
import { ProofInspector } from "../proof/ProofInspector";
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
  const [proof, setProof] = useState<ProofOutlineResponse>();
  const [proofBusy, setProofBusy] = useState(false);
  const [proofError, setProofError] = useState<string>();
  const [selectedProofStep, setSelectedProofStep] = useState<number>();
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
    if (!bootstrap || state.view !== "proof") return;
    const available = bootstrap.proofDeclarations;
    const currentIsAvailable = available.some((item) => item.name === state.proofDeclaration);
    if (!currentIsAvailable && available[0]) {
      dispatch({ type: "patch", value: { proofDeclaration: available[0].name } });
    }
  }, [bootstrap, state.proofDeclaration, state.view]);

  useEffect(() => {
    if (state.view !== "proof" || !state.proofDeclaration) return;
    const controller = new AbortController();
    setProofBusy(true);
    setProofError(undefined);
    setProof(undefined);
    setSelectedProofStep(undefined);
    api.proofOutline(state.proofDeclaration, controller.signal)
      .then((response) => {
        setProof(response);
      })
      .catch((error) => { if (error.name !== "AbortError") setProofError(error.message); })
      .finally(() => { if (!controller.signal.aborted) setProofBusy(false); });
    return () => controller.abort();
  }, [state.proofDeclaration, state.view]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (state.view === "proof" && selectedProofStep !== undefined) { setSelectedProofStep(undefined); return; }
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

  const changeView = (view: "sieve" | "proof") => {
    const selectedProof = selectedNode?.leanName && bootstrap?.proofDeclarations.some((item) => item.name === selectedNode.leanName)
      ? selectedNode.leanName
      : state.proofDeclaration;
    dispatch({ type: "patch", value: { view, proofDeclaration: selectedProof } });
  };

  if (state.view === "proof") {
    return <AppShell theme={theme} view={state.view} onTheme={toggle} onView={changeView}
      controls={<ProofControls bootstrap={bootstrap} declaration={state.proofDeclaration} data={proof}
        state={state} dispatch={dispatch}
        onDeclaration={(proofDeclaration) => dispatch({ type: "patch", value: { proofDeclaration } })} />}
      inspector={<ProofInspector data={proof} selected={selectedProofStep} />}>
      <div className="graph-header proof-header">
        <strong className="graph-title">{proof?.declaration ?? "Proof outline"}</strong>
        {proof && <div className="visible-count"><strong>{proof.outline.retainedNodes.length}</strong><span>steps</span><strong>{proof.outline.condensedEdges.length}</strong><span>connections</span></div>}
      </div>
      <div className="graph-stage" data-loading={proofBusy}>
        {proof && <ProofGraph data={proof} selected={selectedProofStep} showLabels={state.showLabels}
          theme={theme} onSelect={setSelectedProofStep} />}
        {(proofBusy || !bootstrap) && !proof && !proofError && !error && <div className="loading-state"><span className="loading-ring" /></div>}
        {(proofError || error) && <div className="error-state"><span>!</span><p>{proofError ?? error}</p><button onClick={() => location.reload()}>Retry</button></div>}
        {!proofBusy && !proofError && !error && bootstrap?.proofDeclarations.length === 0 && <div className="empty-state">No theorem proof steps were extracted for this corpus.</div>}
      </div>
    </AppShell>;
  }

  return <AppShell theme={theme} view={state.view} onTheme={toggle} onView={changeView}
      controls={<GraphControls state={state} dispatch={dispatch} bootstrap={bootstrap} />}
      inspector={<Inspector node={selectedNode} edge={selectedEdge} witnesses={witnesses} busy={busy}
        onWitnesses={showWitnesses} onCloseDetail={() => setWitnesses(undefined)} />}>
      {data && <GraphHeader data={data} />}
      <div className="graph-stage" data-loading={busy}>
        {data && <GraphCanvas data={data} mostUsed={state.mostUsed} showLabels={state.showLabels}
          selected={state.selected} selectedEdge={state.selectedEdge}
          theme={theme}
          onSelect={selectNode} onSelectEdge={(id) => dispatch({ type: "selectEdge", id })}
          onOpen={selectNode} onOpenEdge={openEdge} />}
        {busy && !data && <div className="loading-state"><span className="loading-ring" /></div>}
        {error && <div className="error-state"><span>!</span><p>{error}</p><button onClick={() => location.reload()}>Retry</button></div>}
        {!busy && !error && data?.nodes.length === 0 && <div className="empty-state">No statements match the current filters.</div>}
      </div>
    </AppShell>;
}
