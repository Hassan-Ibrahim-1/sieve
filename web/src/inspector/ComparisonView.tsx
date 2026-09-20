import type { Comparison } from "../api/types";

export function ComparisonView({ data, onClose }: { data: Comparison[]; onClose: () => void }) {
  const comparison = data[0];
  return <div className="comparison-view">
    <div className="inspector-title"><span>Comparison</span><button onClick={onClose} aria-label="Close comparison">×</button></div>
    <div className="comparison-status">
      <Badge yes={comparison.exactEqual} label="Exact" />
      <Badge yes={comparison.alphaEquivalent} label="α-equivalent" />
      <strong>{Math.round(comparison.similarity.combinedScore * 100)}%</strong>
    </div>
    <DependencyGroup label="Shared" items={comparison.sharedDependencies} tone="shared" />
    <DependencyGroup label="Left only" items={comparison.leftOnlyDependencies} tone="left" />
    <DependencyGroup label="Right only" items={comparison.rightOnlyDependencies} tone="right" />
    <div className="similarity-bars">
      {Object.entries(comparison.similarity).filter(([key]) => key !== "combinedScore").map(([key, value]) => <div key={key}>
        <span>{key.replace(/[A-Z]/g, (letter) => ` ${letter.toLowerCase()}`)}</span>
        <i><b style={{ width: `${value * 100}%` }} /></i><output>{Math.round(value * 100)}</output>
      </div>)}
    </div>
  </div>;
}

function Badge({ yes, label }: { yes: boolean; label: string }) { return <span data-yes={yes}>{yes ? "✓" : "—"} {label}</span>; }
function DependencyGroup({ label, items, tone }: { label: string; items: string[]; tone: string }) {
  return <section className="dependency-group"><h3>{label}<small>{items.length}</small></h3>
    <div>{items.slice(0, 8).map((item) => <code data-tone={tone} key={item}>{item}</code>)}</div>
  </section>;
}
