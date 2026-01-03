import type { Bounds } from "../types/diagram";

/** Get center of a bounds rectangle */
export function getCenter(b: Bounds): { x: number; y: number } {
  return { x: b.x + b.w / 2, y: b.y + b.h / 2 };
}

/** Supported node shapes */
export type NodeShape = "rectangle" | "circle" | "diamond";

/** Get shape from modifiers (default: rectangle) */
export function getShape(modifiers: string[]): NodeShape {
  if (modifiers.includes("circle")) return "circle";
  if (modifiers.includes("diamond")) return "diamond";
  return "rectangle";
}

/** Calculate intersection point of line from center to target with node shape */
export function getEdgePoint(bounds: Bounds, targetX: number, targetY: number, shape: NodeShape = "rectangle"): { x: number; y: number } {
  const cx = bounds.x + bounds.w / 2;
  const cy = bounds.y + bounds.h / 2;
  const dx = targetX - cx;
  const dy = targetY - cy;

  if (dx === 0 && dy === 0) return { x: cx, y: cy };

  if (shape === "circle") {
    // Ellipse intersection
    // x = cx + (w/2) * cos(theta)
    // y = cy + (h/2) * sin(theta)
    const angle = Math.atan2(dy, dx);
    return {
      x: cx + (bounds.w / 2) * Math.cos(angle),
      y: cy + (bounds.h / 2) * Math.sin(angle),
    };
  }

  if (shape === "diamond") {
    // Diamond is just a rotated square (rhombus)
    // We can intersect with the 4 segments connecting the midpoints of the bounding box sides
    // Top: (cx, y), Right: (x+w, cy), Bottom: (cx, y+h), Left: (x, cy)
    // Relative to center: (0, -h/2), (w/2, 0), (0, h/2), (-w/2, 0)

    // Normalize direction to quadrant 1
    // The equation for the line in Q1 is: x/(w/2) + y/(h/2) = 1  =>  2x/w + 2y/h = 1
    // We have point on line P = (dx * t, dy * t)
    // 2(dx * t)/w + 2(dy * t)/h = 1
    // t * (2dx/w + 2dy/h) = 1
    // t = 1 / (2|dx|/w + 2|dy|/h)

    const t = 1 / ((2 * Math.abs(dx)) / bounds.w + (2 * Math.abs(dy)) / bounds.h);

    return {
      x: cx + dx * t,
      y: cy + dy * t,
    };
  }

  // Rectangle (default)
  const halfW = bounds.w / 2;
  const halfH = bounds.h / 2;

  const tRight = halfW / Math.abs(dx);
  const tLeft = halfW / Math.abs(dx);
  const tBottom = halfH / Math.abs(dy);
  const tTop = halfH / Math.abs(dy);

  let t = Infinity;

  if (dx > 0) t = Math.min(t, tRight);
  if (dx < 0) t = Math.min(t, tLeft);
  if (dy > 0) t = Math.min(t, tBottom);
  if (dy < 0) t = Math.min(t, tTop);

  return {
    x: cx + dx * t,
    y: cy + dy * t,
  };
}

/** Check if arrow points to the "from" node (left arrows) */
export function isLeftArrow(arrow: string): boolean {
  return arrow.endsWith("_left");
}

/** Check if the edge should be dashed */
export function isDashed(arrow: string): boolean {
  return arrow === "dotted" || arrow.startsWith("dep_");
}

/** Get the marker type for an edge based on arrow type and direction */
export function getEdgeMarkers(arrow: string): {
  markerStart: string;
  markerEnd: string;
} {
  const leftArrow = isLeftArrow(arrow);
  const arrowAtFrom = leftArrow;
  const baseArrow = arrow.replace("_left", "").replace("_right", "");

  let markerEnd = "";
  let markerStart = "";

  if (baseArrow === "extends") {
    if (arrowAtFrom) markerStart = "url(#triangle-start)";
    else markerEnd = "url(#triangle-end)";
  } else if (baseArrow === "assoc" || baseArrow === "dep") {
    if (arrowAtFrom) markerStart = "url(#arrowhead-start)";
    else markerEnd = "url(#arrowhead-end)";
  } else if (baseArrow === "aggregate") {
    markerStart = "url(#diamond-empty-start)";
  } else if (baseArrow === "compose") {
    markerStart = "url(#diamond-start)";
  }

  return { markerStart, markerEnd };
}
