import type { GraphResponse, RankingMetric } from "../api/types";
import { rankingMetricDetails } from "../analysis/TheoremRankings";

export function GraphHeader({ data, mode, rankingMetric, rankingCount, onMode, onRankingMetric }: {
  data: GraphResponse;
  mode: "graph" | "ranking";
  rankingMetric: RankingMetric;
  rankingCount?: number;
  onMode: (mode: "graph" | "ranking") => void;
  onRankingMetric: (metric: RankingMetric) => void;
}) {
  return <div className="graph-header">
    <div className="graph-heading">
      <strong className="graph-title">{mode === "graph" ? "Dependencies and equivalent statements" : "Theorem rankings"}</strong>
      {mode === "ranking" && <label className="ranking-picker">
        <span>Rank by</span>
        <select aria-label="Ranking metric" value={rankingMetric}
          onChange={(event) => onRankingMetric(event.target.value as RankingMetric)}>
          {(Object.keys(rankingMetricDetails) as RankingMetric[]).map((metric) =>
            <option value={metric} key={metric}>{rankingMetricDetails[metric].label}</option>)}
        </select>
      </label>}
    </div>
    <button type="button" className="analyze-button" data-active={mode === "ranking"}
      onClick={() => onMode(mode === "graph" ? "ranking" : "graph")}>
      {mode === "graph" ? "Analyze graph" : "View graph"}
    </button>
    <div className="visible-count">
      {mode === "ranking" ? <><strong>{rankingCount?.toLocaleString() ?? "—"}</strong><span>theorems</span></> : <>
        <strong>{data.totals.declarations.toLocaleString()}</strong><span>declarations</span>
        <strong>{data.totals.groups.toLocaleString()}</strong><span>groups</span>
        <strong>{data.totals.connections.toLocaleString()}</strong><span>connections</span>
      </>}
    </div>
  </div>;
}
