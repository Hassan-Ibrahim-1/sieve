import { initialState, reducer, stateFromUrl } from "../app/state";

describe("UI state", () => {
  it("loads the flat graph state from the URL", () => {
    const state = stateFromUrl("?mostUsed=1&labels=0&witnessLimit=6&definitions=0&technical=1");
    expect(state).toMatchObject({ mostUsed: true, showLabels: false, witnessLimit: 6 });
    expect(state.filters.includeDefinitions).toBe(false);
    expect(state.filters).not.toHaveProperty("includeTechnical");
  });

  it("loads the proof view and selected theorem from the URL", () => {
    const state = stateFromUrl("?view=proof&proof=Fixture.theorem");
    expect(state.view).toBe("proof");
    expect(state.proofDeclaration).toBe("Fixture.theorem");
  });

  it("loads theorem ranking mode and metric from the URL", () => {
    const state = stateFromUrl("?graphMode=ranking&ranking=bridge");
    expect(state.graphMode).toBe("ranking");
    expect(state.rankingMetric).toBe("bridge");
  });

  it("resets the graph controls", () => {
    const state = reducer({ ...initialState, mostUsed: true, witnessLimit: 8 }, { type: "reset" });
    expect(state).toEqual({ ...initialState, graphResetVersion: 1 });
  });

  it("emits a fresh graph reset signal even when controls are already at their defaults", () => {
    const first = reducer(initialState, { type: "reset" });
    const second = reducer(first, { type: "reset" });
    expect(second.graphResetVersion).toBe(2);
  });
});
