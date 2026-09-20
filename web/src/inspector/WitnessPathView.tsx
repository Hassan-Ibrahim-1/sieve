import type { WitnessResponse } from "../api/types";

export function WitnessPathView({ data, onClose }: { data: WitnessResponse; onClose: () => void }) {
  return <div>
    <div className="inspector-title"><span>Witness paths</span><button onClick={onClose} aria-label="Close witness paths">×</button></div>
    <div className="witness-paths">{data.paths.map((path, index) => <ol key={index}>
      {path.nodes.map((node) => <li key={node}><span />{node}</li>)}
    </ol>)}</div>
  </div>;
}
