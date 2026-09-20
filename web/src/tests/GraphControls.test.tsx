import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { GraphControls } from "../controls/GraphControls";
import { initialState } from "../app/state";

describe("GraphControls", () => {
  it("uses a single graph with an optional most-used emphasis", () => {
    render(<GraphControls state={initialState} dispatch={vi.fn()} onSearchSelect={vi.fn()} />);
    expect(screen.getByRole("checkbox", { name: "Emphasize most used" })).not.toBeChecked();
    expect(screen.queryByRole("combobox", { name: "Graph mode" })).not.toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Visible nodes" })).not.toBeInTheDocument();
    expect(screen.queryByText("Proof steps")).not.toBeInTheDocument();
  });

  it("toggles most-used emphasis", () => {
    const dispatch = vi.fn();
    render(<GraphControls state={initialState} dispatch={dispatch} onSearchSelect={vi.fn()} />);
    fireEvent.click(screen.getByRole("checkbox", { name: "Emphasize most used" }));
    expect(dispatch).toHaveBeenCalledWith({ type: "patch", value: { mostUsed: true } });
  });
});
