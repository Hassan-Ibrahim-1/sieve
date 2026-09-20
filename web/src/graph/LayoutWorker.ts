interface LayoutNode {
  id: string;
  x: number;
  y: number;
  size: number;
  usage: number;
  nodeKind: "group" | "declaration";
  groupId?: string;
}

interface LayoutRequest {
  kind: "grouped-force";
  mostUsed: boolean;
  nodes: LayoutNode[];
  edges: Array<{ source: string; target: string }>;
}

self.onmessage = (event: MessageEvent<LayoutRequest>) => {
  const payload = event.data;
  const groupIds = new Set(payload.nodes.filter((node) => node.nodeKind === "group").map((node) => node.id));
  const units = payload.nodes
    .filter((node) => node.nodeKind === "group" || !node.groupId || !groupIds.has(node.groupId))
    .sort((left, right) => left.id.localeCompare(right.id));
  const points = new Map<string, { x: number; y: number }>();
  const columns = Math.max(1, Math.ceil(Math.sqrt(units.length)));
  const spacing = 170;
  for (const [index, node] of units.entries()) {
    const column = index % columns;
    const row = Math.floor(index / columns);
    points.set(node.id, {
      x: (column - (columns - 1) / 2) * spacing + node.x * 0.08,
      y: (row - (Math.ceil(units.length / columns) - 1) / 2) * spacing + node.y * 0.08,
    });
  }

  const edgePairs = payload.edges
    .map((edge) => [edge.source, edge.target] as const)
    .filter(([source, target]) => points.has(source) && points.has(target));
  const maximumUsage = Math.max(1, ...units.map((node) => node.usage));
  const unitById = new Map(units.map((node) => [node.id, node]));

  for (let iteration = 0; iteration < 180; iteration++) {
    const movement = new Map(units.map((node) => [node.id, { x: 0, y: 0 }]));
    const cellSize = 210;
    const grid = new Map<string, LayoutNode[]>();
    for (const node of units) {
      const point = points.get(node.id)!;
      const key = `${Math.floor(point.x / cellSize)}:${Math.floor(point.y / cellSize)}`;
      const cell = grid.get(key) ?? [];
      cell.push(node);
      grid.set(key, cell);
    }

    for (const node of units) {
      const point = points.get(node.id)!;
      const cellX = Math.floor(point.x / cellSize);
      const cellY = Math.floor(point.y / cellSize);
      for (let dxCell = -1; dxCell <= 1; dxCell++) {
        for (let dyCell = -1; dyCell <= 1; dyCell++) {
          for (const other of grid.get(`${cellX + dxCell}:${cellY + dyCell}`) ?? []) {
            if (other.id <= node.id) continue;
            const otherPoint = points.get(other.id)!;
            const dx = point.x - otherPoint.x || 0.01;
            const dy = point.y - otherPoint.y || 0.01;
            const distance = Math.max(1, Math.hypot(dx, dy));
            const minimum = 95 + node.size + other.size;
            if (distance >= minimum * 1.7) continue;
            const force = Math.max(2, (minimum * minimum) / distance);
            const mx = (dx / distance) * force;
            const my = (dy / distance) * force;
            movement.get(node.id)!.x += mx;
            movement.get(node.id)!.y += my;
            movement.get(other.id)!.x -= mx;
            movement.get(other.id)!.y -= my;
          }
        }
      }
    }

    for (const [source, target] of edgePairs) {
      const left = points.get(source)!;
      const right = points.get(target)!;
      const dx = left.x - right.x;
      const dy = left.y - right.y;
      const distance = Math.max(2, Math.hypot(dx, dy));
      const ideal = 165 + unitById.get(source)!.size + unitById.get(target)!.size;
      const force = (distance - ideal) * 0.08;
      const mx = (dx / distance) * force;
      const my = (dy / distance) * force;
      movement.get(source)!.x -= mx;
      movement.get(source)!.y -= my;
      movement.get(target)!.x += mx;
      movement.get(target)!.y += my;
    }

    const temperature = 15 * (1 - iteration / 180) + 0.5;
    for (const node of units) {
      const point = points.get(node.id)!;
      const move = movement.get(node.id)!;
      const usage = node.usage / maximumUsage;
      const gravity = payload.mostUsed ? 0.006 + usage * 0.035 : 0.006;
      move.x -= point.x * gravity;
      move.y -= point.y * gravity;
      const length = Math.max(1, Math.hypot(move.x, move.y));
      point.x += (move.x / length) * Math.min(length, temperature);
      point.y += (move.y / length) * Math.min(length, temperature);
    }
  }

  const positions: Record<string, { x: number; y: number }> = Object.fromEntries(points);
  const membersByGroup = new Map<string, LayoutNode[]>();
  for (const node of payload.nodes) {
    if (!node.groupId || !groupIds.has(node.groupId)) continue;
    const members = membersByGroup.get(node.groupId) ?? [];
    members.push(node);
    membersByGroup.set(node.groupId, members);
  }
  for (const [groupId, members] of membersByGroup) {
    const center = points.get(groupId)!;
    members.sort((left, right) => left.id.localeCompare(right.id));
    members.forEach((node, index) => {
      let ring = 1;
      let ringStart = 0;
      while (index >= ringStart + ring * 6) {
        ringStart += ring * 6;
        ring += 1;
      }
      const count = Math.min(ring * 6, members.length - ringStart);
      const angle = -Math.PI / 2 + ((index - ringStart) / count) * Math.PI * 2;
      const radius = 8 + ring * 20;
      positions[node.id] = {
        x: center.x + Math.cos(angle) * radius,
        y: center.y + Math.sin(angle) * radius,
      };
    });
  }
  self.postMessage(positions);
};
