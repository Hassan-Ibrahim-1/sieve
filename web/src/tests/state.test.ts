import { initialState, reducer, stateFromUrl } from "../app/state";

describe("UI state", () => {
  it("loads the flat graph state from the URL", () => {
    const state = stateFromUrl("?mostUsed=1&witnessLimit=6&definitions=0&technical=1&pin=decl:a&pin=decl:b");
    expect(state).toMatchObject({ mostUsed: true, witnessLimit: 6 });
    expect(state.filters.includeDefinitions).toBe(false);
    expect(state.filters.includeTechnical).toBe(true);
    expect(state.pins).toEqual(["decl:a", "decl:b"]);
  });

  it("keeps at most two comparison pins", () => {
    let state = reducer(initialState, { type: "pin", id: "decl:a" });
    state = reducer(state, { type: "pin", id: "decl:b" });
    state = reducer(state, { type: "pin", id: "decl:c" });
    expect(state.pins).toEqual(["decl:b", "decl:c"]);
  });

  it("resets the graph controls", () => {
    const state = reducer({ ...initialState, mostUsed: true, witnessLimit: 8 }, { type: "reset" });
    expect(state).toEqual(initialState);
  });
});
