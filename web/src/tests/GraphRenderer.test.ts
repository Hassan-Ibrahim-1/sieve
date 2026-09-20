import { groupRadiusAtZoom, NODE_LABEL_ZOOM_THRESHOLD, nodeLabelsAreVisible } from "../graph/graphZoom";

describe("graph label visibility", () => {
  it("shows labels while close and hides them past the zoom-out threshold", () => {
    expect(nodeLabelsAreVisible(0.5)).toBe(true);
    expect(nodeLabelsAreVisible(NODE_LABEL_ZOOM_THRESHOLD)).toBe(true);
    expect(nodeLabelsAreVisible(NODE_LABEL_ZOOM_THRESHOLD + 0.01)).toBe(false);
  });

  it("scales group circles with the camera zoom", () => {
    expect(groupRadiusAtZoom(40, 1)).toBe(40);
    expect(groupRadiusAtZoom(40, 0.5)).toBe(80);
    expect(groupRadiusAtZoom(40, 2)).toBe(20);
  });
});
