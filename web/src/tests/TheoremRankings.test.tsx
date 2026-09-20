import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import type { RankingResponse } from "../api/types";
import { TheoremRankings } from "../analysis/TheoremRankings";

const data: RankingResponse = {
  schemaVersion: 1,
  corpusFingerprint: "fixture",
  theorems: [
    { id: "decl:Alpha", name: "Alpha", statement: "P", displayStatement: "P", metrics: { loadBearing: 2, connected: 8, bridge: 1 } },
    { id: "decl:Beta", name: "Beta", statement: "Q", displayStatement: "Q", metrics: { loadBearing: 5, connected: 6, bridge: 4 } },
  ],
};

describe("TheoremRankings", () => {
  it("sorts by the chosen graph metric", () => {
    const { rerender } = render(<TheoremRankings data={data} metric="loadBearing" onSelect={vi.fn()} />);
    expect(screen.getAllByRole("button").map((button) => button.textContent)).toEqual([
      expect.stringContaining("Beta"),
      expect.stringContaining("Alpha"),
    ]);

    rerender(<TheoremRankings data={data} metric="connected" onSelect={vi.fn()} />);
    expect(screen.getAllByRole("button").map((button) => button.textContent)).toEqual([
      expect.stringContaining("Alpha"),
      expect.stringContaining("Beta"),
    ]);
  });

  it("selects a theorem through a list row", () => {
    const onSelect = vi.fn();
    render(<TheoremRankings data={data} metric="bridge" selected="decl:Beta" onSelect={onSelect} />);
    const beta = screen.getByRole("button", { name: /Beta/ });
    expect(beta).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("button", { name: /Alpha/ }));
    expect(onSelect).toHaveBeenCalledWith("decl:Alpha");
  });
});
