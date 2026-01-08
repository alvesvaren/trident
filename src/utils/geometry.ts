import type { Bounds } from "../types/diagram";
import { getArrowByName, type ArrowEntry } from "../types/arrows";

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
  const entry = getArrowByName(arrow);
  return entry?.is_left ?? false;
}

/** Check if the edge should be dashed */
export function isDashed(arrow: string): boolean {
  const entry = getArrowByName(arrow);
  return entry?.line_style === "dashed";
}

/** Map head style to SVG marker ID */
function headStyleToMarker(headStyle: string, position: "start" | "end"): string {
  switch (headStyle) {
    case "arrow":
      return `url(#arrowhead-${position})`;
    case "triangle":
      return `url(#triangle-${position})`;
    case "diamond_filled":
      return `url(#diamond-${position})`;
    case "diamond_empty":
      return `url(#diamond-empty-${position})`;
    default:
      return "";
  }
}

/** Get the marker type for an edge based on arrow type and direction */
export function getEdgeMarkers(arrow: string): {
  markerStart: string;
  markerEnd: string;
} {
  const entry = getArrowByName(arrow);
  
  if (!entry) {
    // Fallback for unknown arrows
    return { markerStart: "", markerEnd: "" };
  }

  const isLeft = entry.is_left;
  
  // For left arrows, the visual direction is reversed
  // head_style goes on the "from" side (markerStart)
  // tail_style goes on the "to" side (markerEnd)
  // For right arrows, it's the opposite
  
  let markerStart = "";
  let markerEnd = "";
  
  if (isLeft) {
    // Left arrow: head is at the "from" node (markerStart)
    markerStart = headStyleToMarker(entry.head_style, "start");
    markerEnd = headStyleToMarker(entry.tail_style, "end");
  } else {
    // Right arrow or non-directional: head is at the "to" node (markerEnd)
    markerEnd = headStyleToMarker(entry.head_style, "end");
    markerStart = headStyleToMarker(entry.tail_style, "start");
  }

  return { markerStart, markerEnd };
}
