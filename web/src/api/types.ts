export type GraphMode = "similarity" | "influence" | "connections" | "proof";
export type GraphLevel = "corpus" | "family" | "neighborhood" | "proof";
export type Metric = "recommended" | "familySize" | "directDependents" | "reachableDependents" | "bridgeEvidence" | "proofSize";

export interface UiFilters {
  includeTheorems: boolean;
  includeDefinitions: boolean;
  includeTechnical: boolean;
}

export interface Bootstrap {
  schemaVersion: number;
  corpusFingerprint: string;
  declarationCount: number;
  theoremCount: number;
  graphModes: GraphMode[];
  nodeSizeMetrics: Metric[];
  defaultFilters: UiFilters;
  proofStepsAvailable: boolean;
}

export interface Breadcrumb { level: GraphLevel; id?: string; label: string }
export interface GraphPosition { x: number; y: number }

export interface GraphNode {
  id: string;
  nodeKind: "family" | "declaration" | "proofStep" | "rawProofStep" | "collapsedPath";
  declarationKind?: string;
  statement: string;
  displayStatement: string;
  leanName?: string;
  familyId?: string;
  memberCount: number;
  metrics: Record<Metric | string, number>;
  generated: boolean;
  technical: boolean;
  position: GraphPosition;
  actions: string[];
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  kind: "similarity" | "dependency" | "aggregate" | "condensedProofPath";
  directed: boolean;
  weight: number;
  aggregateCount: number;
  similarity?: number;
  witnessAvailable: boolean;
  collapsedStepCount?: number;
}

export interface GraphResponse {
  schemaVersion: number;
  corpusFingerprint: string;
  scope: { level: GraphLevel; id?: string; breadcrumbs: Breadcrumb[] };
  totals: { matching: number; returned: number; truncated: boolean };
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface ProofResponse {
  declaration: string;
  complete: boolean;
  truncationReason?: string;
  conclusionStep?: number;
  graph: GraphResponse;
}

export interface SimilarityComponents {
  dependencyJaccard: number;
  kindHistogramCosine: number;
  sizeRatio: number;
  depthRatio: number;
  combinedScore: number;
}

export interface Comparison {
  left: string;
  right: string;
  layer: "statement" | "proof";
  exactEqual: boolean;
  alphaEquivalent: boolean;
  similarity: SimilarityComponents;
  sharedDependencies: string[];
  leftOnlyDependencies: string[];
  rightOnlyDependencies: string[];
}

export interface WitnessResponse {
  source: string;
  target: string;
  paths: Array<{ nodes: string[]; layers: string[]; totalWeight: number }>;
}

export interface SearchResponse { results: GraphNode[] }
