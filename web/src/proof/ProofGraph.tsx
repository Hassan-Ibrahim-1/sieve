import type { ProofOutlineResponse } from "../api/types";
import { GraphSurface } from "../graph/GraphSurface";
import type { Theme } from "../theme/useTheme";
import { ProofGraphRenderer } from "./ProofGraphRenderer";

export function ProofGraph({ data, selected, showLabels, theme, onSelect }: {
  data: ProofOutlineResponse;
  selected?: number;
  showLabels: boolean;
  theme: Theme;
  onSelect: (step?: number) => void;
}) {
  if (data.outline.retainedNodes.length === 0) {
    return <div className="empty-state">No proof outline could be retained for this theorem.</div>;
  }
  return <GraphSurface label="Interactive proof-step graph">
    <ProofGraphRenderer data={data} selected={selected} showLabels={showLabels} theme={theme} onSelect={onSelect} />
  </GraphSurface>;
}
