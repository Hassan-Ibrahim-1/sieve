import type { GraphResponse } from "../api/types";
import type { Theme } from "../theme/useTheme";
import { GraphRenderer } from "./GraphRenderer";
import { GraphSurface } from "./GraphSurface";

interface Props {
  data: GraphResponse;
  mostUsed: boolean;
  showLabels: boolean;
  resetVersion: number;
  selected?: string;
  selectedEdge?: string;
  theme: Theme;
  onSelect: (id?: string) => void;
  onSelectEdge: (id?: string) => void;
  onOpen: (id: string) => void;
  onOpenEdge: (id: string) => void;
}

export function GraphCanvas(props: Props) {
  return <GraphSurface label="Interactive mathematical graph"
    onKeyDown={(event) => {
      if (event.key === "Enter" && props.selected) props.onOpen(props.selected);
    }}>
    <GraphRenderer {...props} />
  </GraphSurface>;
}
