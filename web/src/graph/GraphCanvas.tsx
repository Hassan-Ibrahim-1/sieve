import { SigmaContainer } from "@react-sigma/core";
import { EdgeArrowProgram, type NodeLabelDrawingFunction } from "sigma/rendering";
import type { Settings } from "sigma/settings";
import type { GraphResponse, Metric } from "../api/types";
import type { Theme } from "../theme/useTheme";
import { GraphRenderer } from "./GraphRenderer";

interface Props {
  data: GraphResponse;
  metric: Metric;
  selected?: string;
  selectedEdge?: string;
  pins: string[];
  search: string;
  layout: "force" | "layered" | "clustered";
  theme: Theme;
  onSelect: (id?: string) => void;
  onSelectEdge: (id?: string) => void;
  onOpen: (id: string) => void;
  onOpenEdge: (id: string) => void;
}

export function GraphCanvas(props: Props) {
  return <div className="graph-canvas" tabIndex={0} aria-label="Interactive mathematical graph"
    onKeyDown={(event) => {
      if (event.key === "Enter" && props.selected) props.onOpen(props.selected);
    }}>
    <SigmaContainer settings={graphSettings}>
      <GraphRenderer {...props} />
    </SigmaContainer>
    <div className="canvas-tools" aria-hidden="true"><span>+</span><span>−</span></div>
  </div>;
}

const drawNodeLabel: NodeLabelDrawingFunction = (context, data, settings) => {
  if (!data.label) return;
  const dark = (data as typeof data & { labelTheme?: Theme }).labelTheme === "dark";
  const fontSize = settings.labelSize;
  context.font = `${settings.labelWeight} ${fontSize}px ${settings.labelFont}`;
  const width = Math.ceil(context.measureText(data.label).width) + 12;
  const height = 19;
  const x = Math.round(data.x - width / 2);
  const y = Math.round(data.y + data.size + 7);

  context.beginPath();
  context.roundRect(x, y, width, height, 5);
  context.fillStyle = dark ? "rgba(17, 21, 19, .94)" : "rgba(255, 255, 255, .96)";
  context.fill();
  context.strokeStyle = dark ? "rgba(66, 199, 147, .24)" : "rgba(24, 33, 30, .15)";
  context.lineWidth = 1;
  context.stroke();

  context.fillStyle = dark ? "#e9efeb" : "#18211e";
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillText(data.label, data.x, y + height / 2 + 0.5);
};

// This object must stay referentially stable. React Sigma reconstructs its WebGL
// renderer whenever settings change, which is unsafe during rapid slider updates.
const graphSettings: Partial<Settings> = {
  allowInvalidContainer: true,
  renderEdgeLabels: false,
  labelFont: "IBM Plex Sans, ui-sans-serif, system-ui",
  labelSize: 10,
  labelWeight: "500",
  labelColor: { color: "#18211e" },
  labelDensity: 0.55,
  labelGridCellSize: 150,
  labelRenderedSizeThreshold: 7,
  defaultDrawNodeLabel: drawNodeLabel,
  defaultDrawNodeHover: drawNodeLabel,
  stagePadding: 95,
  zIndex: true,
  edgeProgramClasses: { arrow: EdgeArrowProgram },
  defaultEdgeType: "line",
};
