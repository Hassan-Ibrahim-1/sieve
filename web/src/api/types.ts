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
  defaultFilters: UiFilters;
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
