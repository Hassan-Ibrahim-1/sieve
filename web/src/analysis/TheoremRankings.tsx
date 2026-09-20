import { useMemo } from "react";
import type { RankingMetric, RankingResponse } from "../api/types";

export const rankingMetricDetails: Record<RankingMetric, { label: string; description: string }> = {
  loadBearing: {
    label: "Load-bearing",
    description: "Direct dependents — theorems and definitions that immediately rely on this theorem.",
  },
  connected: {
    label: "Connected",
    description: "Unique direct dependencies plus dependents in the visible dependency graph.",
  },
  bridge: {
    label: "Bridges",
    description: "Betweenness — how often this theorem lies on shortest dependency paths between distant areas.",
  },
};

export function TheoremRankings({ data, metric, selected, onSelect }: {
  data: RankingResponse;
  metric: RankingMetric;
  selected?: string;
  onSelect: (id: string) => void;
}) {
  const ranked = useMemo(() => data.theorems.slice().sort((left, right) =>
    right.metrics[metric] - left.metrics[metric]
      || right.metrics.loadBearing - left.metrics.loadBearing
      || left.name.localeCompare(right.name)), [data, metric]);
  const details = rankingMetricDetails[metric];

  if (ranked.length === 0) {
    return <div className="empty-state">No theorems match the current filters.</div>;
  }

  return <section className="ranking-view" aria-label="Theorem rankings">
    <div className="ranking-intro">
      <div><span>Ranked by</span><strong>{details.label}</strong></div>
      <p>{details.description}</p>
    </div>
    <ol className="ranking-list">
      {ranked.map((theorem, index) => <li key={theorem.id}>
        <button type="button" className="ranking-row" aria-pressed={selected === theorem.id}
          onClick={() => onSelect(theorem.id)}>
          <span className="ranking-position">{index + 1}</span>
          <span className="ranking-theorem">
            <code>{theorem.name}</code>
            <span>{theorem.displayStatement}</span>
          </span>
          <span className="ranking-score">
            <strong>{formatMetric(theorem.metrics[metric], metric)}</strong>
            <span>{details.label}</span>
          </span>
        </button>
      </li>)}
    </ol>
  </section>;
}

function formatMetric(value: number, metric: RankingMetric) {
  if (metric !== "bridge") return value.toLocaleString();
  return new Intl.NumberFormat(undefined, {
    notation: value >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: value < 10 ? 2 : 1,
  }).format(value);
}
