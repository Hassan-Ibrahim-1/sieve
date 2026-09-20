import { SigmaContainer } from "@react-sigma/core";
import type { KeyboardEventHandler, ReactNode } from "react";
import { EdgeArrowProgram, NodeCircleProgram, type NodeLabelDrawingFunction } from "sigma/rendering";
import type { Settings } from "sigma/settings";
import type { Theme } from "../theme/useTheme";

export function GraphSurface({ label, children, onKeyDown }: {
  label: string;
  children: ReactNode;
  onKeyDown?: KeyboardEventHandler<HTMLDivElement>;
}) {
  return <div className="graph-canvas" tabIndex={0} aria-label={label} onKeyDown={onKeyDown}>
    <SigmaContainer settings={graphSettings}>{children}</SigmaContainer>
    <div className="canvas-tools" aria-hidden="true"><span>+</span><span>−</span></div>
  </div>;
}

const drawNodeLabel: NodeLabelDrawingFunction = (context, data, settings) => {
  if (!settings.renderLabels || !data.label) return;
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

// Groups already have a dedicated canvas treatment. Repainting their large
// WebGL disk on hover would cover the member nodes inside them.
class GroupHoverProgram extends NodeCircleProgram {
  render() {}
}

// This object must stay referentially stable. React Sigma reconstructs its WebGL
// renderer whenever settings change, which is unsafe during rapid updates.
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
  nodeProgramClasses: { group: NodeCircleProgram },
  nodeHoverProgramClasses: { group: GroupHoverProgram },
  edgeProgramClasses: { arrow: EdgeArrowProgram },
  defaultEdgeType: "line",
};
