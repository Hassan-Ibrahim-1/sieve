import { initialState, reducer, stateFromUrl } from "../app/state";

describe("UI state", () => {
  it("loads shareable graph state from the URL", () => {
    const state = stateFromUrl("?mode=influence&level=family&scope=family-a&depth=4&limit=120&definitions=0&technical=1&pin=decl:a&pin=decl:b");
    expect(state).toMatchObject({ mode: "influence", level: "family", scope: "family-a", depth: 4, limit: 120 });
    expect(state.filters.includeDefinitions).toBe(false);
    expect(state.filters.includeTechnical).toBe(true);
    expect(state.pins).toEqual(["decl:a", "decl:b"]);
  });

  it("restores a shareable raw proof view", () => {
    const state = stateFromUrl("?mode=proof&level=proof&scope=family-1&proof=decl%3AFtc.theorem&proofDetail=raw&proofPath=edge-7");
    expect(state.mode).toBe("proof");
    expect(state.proofDeclaration).toBe("decl:Ftc.theorem");
    expect(state.proofDetail).toBe("raw");
    expect(state.proofPath).toBe("edge-7");
  });

  it("keeps at most two comparison pins", () => {
    let state = reducer(initialState, { type: "pin", id: "decl:a" });
    state = reducer(state, { type: "pin", id: "decl:b" });
    state = reducer(state, { type: "pin", id: "decl:c" });
    expect(state.pins).toEqual(["decl:b", "decl:c"]);
  });

  it("resets controls without changing the semantic location", () => {
    const state = reducer({ ...initialState, mode: "connections", level: "family", scope: "family-a", limit: 150 }, { type: "reset" });
    expect(state).toMatchObject({ mode: "connections", level: "family", scope: "family-a", limit: 80 });
  });
});
