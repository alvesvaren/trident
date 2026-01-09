import type { Bounds } from "../types/diagram";
import { getArrowByName } from "../types/arrows";

/** Helper to clamp a value between min and max */
function clamp(val: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, val));
}

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

/** Get target point within a node (the "spine" for rectangles) */
export function getTargetPoint(bounds: Bounds, otherCenter: { x: number; y: number }, shape: NodeShape = "rectangle"): { x: number; y: number } {
  const cx = bounds.x + bounds.w / 2;
  const cy = bounds.y + bounds.h / 2;

  if (shape !== "rectangle") {
    return { x: cx, y: cy };
  }

  // Spine logic for rectangles: 30% leeway in the longer dimension
  if (bounds.w > bounds.h) {
    const leeway = bounds.w * 0.3;
    return { x: clamp(otherCenter.x, cx - leeway, cx + leeway), y: cy };
  } else {
    const leeway = bounds.h * 0.3;
    return { x: cx, y: clamp(otherCenter.y, cy - leeway, cy + leeway) };
  }
}

/** Get intersection point of line from start to target with shape boundary */
export function getEdgePoint(
  bounds: Bounds,
  targetX: number,
  targetY: number,
  shape: NodeShape = "rectangle",
  offset: number = 0,
  source?: { x: number; y: number }
): { x: number; y: number } {
  const start = source || getCenter(bounds);
  const dx = targetX - start.x;
  const dy = targetY - start.y;

  if (dx === 0 && dy === 0) return start;

  let intersection: { x: number; y: number };

  if (shape === "circle") {
    const angle = Math.atan2(dy, dx);
    intersection = {
      x: (source?.x ?? (bounds.x + bounds.w / 2)) + (bounds.w / 2) * Math.cos(angle),
      y: (source?.y ?? (bounds.y + bounds.h / 2)) + (bounds.h / 2) * Math.sin(angle),
    };
  } else if (shape === "diamond") {
    const t = 1 / ((2 * Math.abs(dx)) / bounds.w + (2 * Math.abs(dy)) / bounds.h);
    intersection = { x: start.x + dx * t, y: start.y + dy * t };
  } else {
    const halfW = bounds.w / 2;
    const halfH = bounds.h / 2;
    const cx = bounds.x + halfW;
    const cy = bounds.y + halfH;
    
    let t = Infinity;
    if (dx > 0) t = Math.min(t, (cx + halfW - start.x) / dx);
    if (dx < 0) t = Math.min(t, (cx - halfW - start.x) / dx);
    if (dy > 0) t = Math.min(t, (cy + halfH - start.y) / dy);
    if (dy < 0) t = Math.min(t, (cy - halfH - start.y) / dy);
    
    intersection = { x: start.x + dx * t, y: start.y + dy * t };
  }

  if (offset !== 0) {
    const length = Math.sqrt(dx * dx + dy * dy);
    if (length > 0) {
      const unitX = dx / length;
      const unitY = dy / length;
      return { x: intersection.x + unitX * offset, y: intersection.y + unitY * offset };
    }
  }

  return intersection;
}

/** Find optimal connection points with geometric intelligence and aesthetic balance */
export function getOptimalConnectionPoints(
  fromBounds: Bounds,
  toBounds: Bounds,
  fromShape: NodeShape = "rectangle",
  toShape: NodeShape = "rectangle",
  startOffset: number = 0,
  endOffset: number = 0
): { start: { x: number; y: number }; end: { x: number; y: number } } {
  const fromCenter = getCenter(fromBounds);
  const toCenter = getCenter(toBounds);

  // 1. Get spine-based target points for both nodes
  const targetA = getTargetPoint(fromBounds, toCenter, fromShape);
  const targetB = getTargetPoint(toBounds, fromCenter, toShape);

  // 2. Find boundary intersections for the line between targets
  let pA = getEdgePoint(fromBounds, targetB.x, targetB.y, fromShape, 0, targetA);
  let pB = getEdgePoint(toBounds, targetA.x, targetA.y, toShape, 0, targetB);

  // 3. Smart corner snapping for diagonal arrows (only for rectangles)
  if (fromShape === "rectangle") {
    pA = smartCornerSnap(pA, targetB, fromBounds);
  }
  if (toShape === "rectangle") {
    pB = smartCornerSnap(pB, targetA, toBounds);
  }

  // 4. Apply arrow head offsets
  let start = pA;
  let end = pB;

  if (startOffset !== 0 || endOffset !== 0) {
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len > 0) {
      if (startOffset !== 0) {
        start = { x: start.x + (dx / len) * startOffset, y: start.y + (dy / len) * startOffset };
      }
      if (endOffset !== 0) {
        // Offset is applied by pulling the endpoint back towards the start
        end = { x: end.x - (dx / len) * endOffset, y: end.y - (dy / len) * endOffset };
      }
    }
  }

  return { start, end };
}

/** Smart corner snapping for diagonal arrows */
function smartCornerSnap(
  point: { x: number; y: number },
  target: { x: number; y: number },
  bounds: Bounds
): { x: number; y: number } {
  const { x, y, w, h } = bounds;
  const corners = [
    { x, y }, { x: x + w, y }, { x: x + w, y: y + h }, { x, y: y + h }
  ];

  // Ugly check: don't snap if it's almost perpendicular to an axis
  const dx = Math.abs(target.x - point.x);
  const dy = Math.abs(target.y - point.y);
  const aspectRatio = Math.max(dx, dy) > 0 ? Math.min(dx, dy) / Math.max(dx, dy) : 0;

  if (aspectRatio < 0.4) return point;

  // "Shorter that way" check: only snap if corner is closer to target than current intersection
  const currentDistSq = Math.pow(target.x - point.x, 2) + Math.pow(target.y - point.y, 2);
  let bestPoint = point;
  let minDistSq = currentDistSq;

  for (const corner of corners) {
    const cornerDistSq = Math.pow(target.x - corner.x, 2) + Math.pow(target.y - corner.y, 2);
    if (cornerDistSq < minDistSq) {
      minDistSq = cornerDistSq;
      bestPoint = corner;
    }
  }

  return bestPoint;
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
function headStyleToMarker(headStyle: string): string {
  switch (headStyle) {
    case "arrow":
      return `url(#arrowhead)`;
    case "rounded_arrow":
      return `url(#rounded-arrowhead)`;
    case "triangle":
      return `url(#triangle)`;
    case "diamond_filled":
      return `url(#diamond)`;
    case "diamond_empty":
      return `url(#diamond-empty)`;
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
    return { markerStart: "", markerEnd: "" };
  }

  const isLeft = entry.is_left;
  const isDiamond = entry.head_style === "diamond_filled" || entry.head_style === "diamond_empty";

  let markerStart = "";
  let markerEnd = "";

  if (isDiamond) {
    if (isLeft) {
      markerEnd = headStyleToMarker(entry.head_style);
    } else {
      markerStart = headStyleToMarker(entry.head_style);
    }
  } else {
    if (isLeft) {
      markerStart = headStyleToMarker(entry.head_style);
    } else {
      markerEnd = headStyleToMarker(entry.head_style);
    }
  }

  return { markerStart, markerEnd };
}
