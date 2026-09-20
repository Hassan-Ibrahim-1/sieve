import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { GraphControls } from "../controls/GraphControls";
import { initialState } from "../app/state";

describe("GraphControls", () => {
  it("uses a single graph with an optional most-used emphasis", () => {
    render(<GraphControls state={initialState} dispatch={vi.fn()} />);
    expect(screen.getByRole("checkbox", { name: "Emphasize most used" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Show labels" })).toBeChecked();
    expect(screen.queryByRole("combobox", { name: "Graph mode" })).not.toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Visible nodes" })).not.toBeInTheDocument();
    expect(screen.queryByText("Proof steps")).not.toBeInTheDocument();
    expect(screen.queryByRole("searchbox")).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Technical declarations" })).not.toBeInTheDocument();
  });

  it("toggles graph labels", () => {
    const dispatch = vi.fn();
    render(<GraphControls state={initialState} dispatch={dispatch} />);
    fireEvent.click(screen.getByRole("checkbox", { name: "Show labels" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "patch", value: { showLabels: false } });
  });

  it("toggles most-used emphasis", () => {
    const dispatch = vi.fn();
    render(<GraphControls state={initialState} dispatch={dispatch} />);
    fireEvent.click(screen.getByRole("checkbox", { name: "Emphasize most used" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "patch", value: { mostUsed: true } });
  });

  it("dispatches a graph reset", () => {
    const dispatch = vi.fn();
    render(<GraphControls state={initialState} dispatch={dispatch} />);
    fireEvent.click(screen.getByRole("button", { name: "Reset graph" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "reset" });
  });

  it("hides canvas-only controls while analyzing", () => {
    render(<GraphControls state={{ ...initialState, graphMode: "ranking" }} dispatch={vi.fn()} />);
    expect(screen.getByText("Analysis")).toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Show labels" })).not.toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Maximum witness paths" })).not.toBeInTheDocument();
  });
});
