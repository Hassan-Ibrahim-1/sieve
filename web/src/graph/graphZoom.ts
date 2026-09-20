export const NODE_LABEL_ZOOM_THRESHOLD = 1.25;

export function nodeLabelsAreVisible(cameraRatio: number) {
  return cameraRatio <= NODE_LABEL_ZOOM_THRESHOLD;
}

export function groupRadiusAtZoom(radius: number, cameraRatio: number) {
  return radius / cameraRatio;
}
