export interface UiFilters {
  includeTheorems: boolean;
  includeDefinitions: boolean;
}

export interface Bootstrap {
  schemaVersion: number;
  corpusFingerprint: string;
  declarationCount: number;
  theoremCount: number;
  proofDeclarations: ProofDeclaration[];
  defaultFilters: UiFilters;
}

export interface ProofDeclaration {
  name: string;
}

export interface ProofContextEntry {
  id: string;
  userName: string;
  kind: string;
  binderInfo: string;
  type: string;
  value: string | null;
}

export interface ProofOutlineNode {
  rawStepId: number;
  kind: string;
  proposition: string;
  context: ProofContextEntry[];
  hypothesisReferences: string[];
  namedReferences: string[];
}

export interface ProofOutlineEdge {
  source: number;
  target: number;
  pathCount: number;
  maximumRawPathLength: number;
  rawStepPath: number[];
}

export interface ProofOutlineResponse {
  declaration: string;
  statement: string;
  outline: {
    algorithm: string;
    retentionRules: string[];
    complete: boolean;
    truncationReason: string | null;
    visitedTerms: number;
    conclusionStep: number | null;
    retainedNodes: ProofOutlineNode[];
    condensedEdges: ProofOutlineEdge[];
    rawStepCount: number;
    rawEdgeCount: number;
    candidateNodeCount: number;
  };
}

export interface GraphPosition { x: number; y: number }

export interface GraphNode {
  id: string;
  nodeKind: "group" | "declaration";
  declarationKind?: string;
  statement: string;
  displayStatement: string;
  leanName?: string;
  groupId?: string;
  memberCount: number;
  memberIds: string[];
  metrics: Record<string, number>;
  generated: boolean;
  technical: boolean;
  position: GraphPosition;
  actions: string[];
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  kind: "dependency";
  directed: true;
  weight: number;
  aggregateCount: number;
  statementCount: number;
  proofCount: number;
  witnessAvailable: boolean;
}

export interface GraphResponse {
  schemaVersion: number;
  corpusFingerprint: string;
  totals: { declarations: number; groups: number; connections: number };
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export type RankingMetric = "loadBearing" | "connected" | "bridge";

export interface TheoremRanking {
  id: string;
  name: string;
  statement: string;
  displayStatement: string;
  metrics: Record<RankingMetric, number>;
}

export interface RankingResponse {
  schemaVersion: number;
  corpusFingerprint: string;
  theorems: TheoremRanking[];
}

export interface WitnessResponse {
  source: string;
  target: string;
  paths: Array<{ nodes: string[]; layers: string[]; totalWeight: number }>;
}
