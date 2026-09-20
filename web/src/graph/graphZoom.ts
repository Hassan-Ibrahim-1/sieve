export const NODE_LABEL_ZOOM_THRESHOLD = 1.25;

export function nodeLabelsAreVisible(cameraRatio: number) {
  return cameraRatio <= NODE_LABEL_ZOOM_THRESHOLD;
}

/** Keep proof-node radii proportional to their separation while zooming out. */
export function collisionSafeNodeSizeRatio(cameraRatio: number) {
  const ratio = Math.max(cameraRatio, Number.EPSILON);
  return ratio <= 1 ? Math.sqrt(ratio) : ratio;
}

export function groupRadiusAtZoom(radius: number, cameraRatio: number) {
  return radius / cameraRatio;
}
