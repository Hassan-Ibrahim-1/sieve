import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { GraphControls } from "../controls/GraphControls";
import { initialState } from "../app/state";

describe("GraphControls", () => {
  it("shows only the active mode-specific control", () => {
    render(<GraphControls state={initialState} dispatch={vi.fn()} onSearchSelect={vi.fn()} />);
    expect(screen.getByRole("combobox", { name: "Graph mode" })).toHaveValue("similarity");
    expect(screen.getByRole("slider", { name: "Similarity threshold" })).toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Neighborhood depth" })).not.toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Proof steps" })).not.toBeInTheDocument();
  });

  it("offers proof mode after a theorem makes it available", () => {
    render(<GraphControls state={{ ...initialState, proofDeclaration: "decl:a" }} dispatch={vi.fn()} onSearchSelect={vi.fn()} />);
    expect(screen.getByRole("option", { name: "Proof steps" })).toBeInTheDocument();
  });

  it("changes graph mode through the select", () => {
    const dispatch = vi.fn();
    render(<GraphControls state={initialState} dispatch={dispatch} onSearchSelect={vi.fn()} />);
    fireEvent.change(screen.getByRole("combobox", { name: "Graph mode" }), { target: { value: "influence" } });
    expect(dispatch).toHaveBeenCalledWith({ type: "mode", mode: "influence" });
  });
});
