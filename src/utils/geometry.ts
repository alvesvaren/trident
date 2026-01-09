import type { Bounds } from "../types/diagram";
import { getArrowByName } from "../types/arrows";

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

/** Get the closest point on a node's boundary to a target point */
export function getClosestBoundaryPoint(bounds: Bounds, targetX: number, targetY: number, shape: NodeShape = "rectangle"): { x: number; y: number } {
  const { x, y, w, h } = bounds;
  const cx = x + w / 2;
  const cy = y + h / 2;

  switch (shape) {
    case "circle": {
      // For circles, the closest point is along the radius to the target
      const radius = Math.min(w, h) / 2;
      const angle = Math.atan2(targetY - cy, targetX - cx);
      return {
        x: cx + radius * Math.cos(angle),
        y: cy + radius * Math.sin(angle),
      };
    }

    case "diamond": {
      // For diamonds, project onto the diamond shape
      // Diamond is defined by the intersection of |x-cx| + |y-cy| = max(w,h)/2
      const dx = targetX - cx;
      const dy = targetY - cy;
      const scale = Math.min(w, h) / 2 / Math.max(Math.abs(dx), Math.abs(dy));
      return {
        x: cx + dx * scale,
        y: cy + dy * scale,
      };
    }

    case "rectangle":
    default: {
      // For rectangles, clamp the target point to the rectangle bounds
      return {
        x: Math.max(x, Math.min(x + w, targetX)),
        y: Math.max(y, Math.min(y + h, targetY)),
      };
    }
  }
}

/** Calculate intersection point of line from center to target with node shape */
export function getEdgePoint(bounds: Bounds, targetX: number, targetY: number, shape: NodeShape = "rectangle", offset: number): { x: number; y: number } {
  const cx = bounds.x + bounds.w / 2;
  const cy = bounds.y + bounds.h / 2;
  const dx = targetX - cx;
  const dy = targetY - cy;

  if (dx === 0 && dy === 0) return { x: cx, y: cy };

  let intersection: { x: number; y: number };

  if (shape === "circle") {
    // Ellipse intersection
    // x = cx + (w/2) * cos(theta)
    // y = cy + (h/2) * sin(theta)
    const angle = Math.atan2(dy, dx);
    intersection = {
      x: cx + (bounds.w / 2) * Math.cos(angle),
      y: cy + (bounds.h / 2) * Math.sin(angle),
    };
  } else if (shape === "diamond") {
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

    intersection = {
      x: cx + dx * t,
      y: cy + dy * t,
    };
  } else {
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

    intersection = {
      x: cx + dx * t,
      y: cy + dy * t,
    };
  }

  // Apply offset if specified
  if (offset !== 0) {
    const length = Math.sqrt(dx * dx + dy * dy);
    if (length > 0) {
      const unitX = dx / length;
      const unitY = dy / length;
      return {
        x: intersection.x + unitX * offset,
        y: intersection.y + unitY * offset,
      };
    }
  }

  return intersection;
}

/** Find balanced connection points that blend optimal geometry with center-to-center aesthetics */
export function getOptimalConnectionPoints(
  fromBounds: Bounds,
  toBounds: Bounds,
  fromShape: NodeShape = "rectangle",
  toShape: NodeShape = "rectangle",
  startOffset: number = 0,
  endOffset: number = 0
): { start: { x: number; y: number }; end: { x: number; y: number } } {
  // Get centers for reference
  const fromCenter = getCenter(fromBounds);
  const toCenter = getCenter(toBounds);

  // Get center-to-center connection points
  const centerStart = getEdgePoint(fromBounds, toCenter.x, toCenter.y, fromShape, 0);
  const centerEnd = getEdgePoint(toBounds, fromCenter.x, fromCenter.y, toShape, 0);

  // Get geometrically optimal points using continuous optimization
  const fromSegments = getShapeBoundarySegments(fromBounds, fromShape);
  const toSegments = getShapeBoundarySegments(toBounds, toShape);

  // Find standard geometric optimal
  let minDistance = Infinity;
  let standardOptimal = { start: { x: 0, y: 0 }, end: { x: 0, y: 0 } };

  for (const fromSeg of fromSegments) {
    for (const toSeg of toSegments) {
      const { point1, point2, distance } = getClosestPointsBetweenSegments(fromSeg, toSeg);
      if (distance < minDistance) {
        minDistance = distance;
        standardOptimal = { start: point1, end: point2 };
      }
    }
  }

  // Find leftmost and rightmost optimal connections
  let leftmostOptimal = { start: { x: Infinity, y: 0 }, end: { x: Infinity, y: 0 } };
  let rightmostOptimal = { start: { x: -Infinity, y: 0 }, end: { x: -Infinity, y: 0 } };

  for (const fromSeg of fromSegments) {
    for (const toSeg of toSegments) {
      const { point1, point2, distance } = getClosestPointsBetweenSegments(fromSeg, toSeg);
      if (distance < minDistance * 1.05) { // Allow slightly longer for variety
        if (point1.x < leftmostOptimal.start.x) {
          leftmostOptimal = { start: point1, end: point2 };
        }
        if (point1.x > rightmostOptimal.start.x) {
          rightmostOptimal = { start: point1, end: point2 };
        }
      }
    }
  }

  // Average the three optimal points for balanced result
  const optimalStart = {
    x: (standardOptimal.start.x + leftmostOptimal.start.x + rightmostOptimal.start.x) / 3,
    y: (standardOptimal.start.y + leftmostOptimal.start.y + rightmostOptimal.start.y) / 3
  };

  const optimalEnd = {
    x: (standardOptimal.end.x + leftmostOptimal.end.x + rightmostOptimal.end.x) / 3,
    y: (standardOptimal.end.y + leftmostOptimal.end.y + rightmostOptimal.end.y) / 3
  };

  // Balanced weighted average: 25% geometric optimal + 75% center-based
  let connectionStart = {
    x: optimalStart.x * 0.25 + centerStart.x * 0.75,
    y: optimalStart.y * 0.25 + centerStart.y * 0.75
  };

  let connectionEnd = {
    x: optimalEnd.x * 0.25 + centerEnd.x * 0.75,
    y: optimalEnd.y * 0.25 + centerEnd.y * 0.75
  };

  // No special parallel handling - let the weighted average do its work

  // Smart corner snapping: snap if close to corner AND angle of attack is shallow
  connectionStart = smartCornerSnap(connectionStart, connectionEnd, fromBounds);
  connectionEnd = smartCornerSnap(connectionEnd, connectionStart, toBounds);

  // Apply offsets if specified
  let finalStart = connectionStart;
  let finalEnd = connectionEnd;

  if (startOffset !== 0) {
    const sdx = finalEnd.x - finalStart.x;
    const sdy = finalEnd.y - finalStart.y;
    const slength = Math.sqrt(sdx * sdx + sdy * sdy);
    if (slength > 0) {
      const unitX = sdx / slength;
      const unitY = sdy / slength;
      finalStart = {
        x: finalStart.x + unitX * startOffset,
        y: finalStart.y + unitY * startOffset,
      };
    }
  }

  if (endOffset !== 0) {
    const edx = finalStart.x - finalEnd.x;
    const edy = finalStart.y - finalEnd.y;
    const elength = Math.sqrt(edx * edx + edy * edy);
    if (elength > 0) {
      const unitX = edx / elength;
      const unitY = edy / elength;
      finalEnd = {
        x: finalEnd.x + unitX * endOffset,
        y: finalEnd.y + unitY * endOffset,
      };
    }
  }

  return { start: finalStart, end: finalEnd };
}

/** Line segment representation */
interface LineSegment {
  start: { x: number; y: number };
  end: { x: number; y: number };
}

/** Smart corner snapping: snap if close to corner AND arrow is diagonal */
function smartCornerSnap(
  point: { x: number; y: number },
  otherEnd: { x: number; y: number },
  bounds: Bounds
): { x: number; y: number } {
  const { x, y, w, h } = bounds;
  const corners = [
    { x: x, y: y }, // top-left
    { x: x + w, y: y }, // top-right
    { x: x + w, y: y + h }, // bottom-right
    { x: x, y: y + h } // bottom-left
  ];

  // Snap if close to any corner (within 50% of smaller dimension)
  const threshold = Math.min(w, h) * 0.5;

  // Check if arrow is diagonal (not extremely horizontal/vertical)
  const arrowDx = Math.abs(otherEnd.x - point.x);
  const arrowDy = Math.abs(otherEnd.y - point.y);
  const maxArrowDim = Math.max(arrowDx, arrowDy);
  const minArrowDim = Math.min(arrowDx, arrowDy);
  const arrowAspectRatio = maxArrowDim > 0 ? minArrowDim / maxArrowDim : 0;

  // Only snap if arrow has significant diagonal component
  if (arrowAspectRatio < 0.4) {
    return point; // Don't snap if arrow is too straight
  }

  for (const corner of corners) {
    const distanceToCorner = Math.sqrt(
      Math.pow(point.x - corner.x, 2) +
      Math.pow(point.y - corner.y, 2)
    );

    if (distanceToCorner <= threshold) {
      return corner; // Snap to corner
    }
  }

  return point; // No snapping needed
}


/** Get boundary segments that make up a shape's outline */
function getShapeBoundarySegments(bounds: Bounds, shape: NodeShape): LineSegment[] {
  const { x, y, w, h } = bounds;

  switch (shape) {
    case "circle": {
      // Approximate circle with multiple segments
      const cx = x + w / 2;
      const cy = y + h / 2;
      const radius = Math.min(w, h) / 2;
      const segments: LineSegment[] = [];
      const numSegments = 16; // More segments for smoother approximation

      for (let i = 0; i < numSegments; i++) {
        const angle1 = (i * 2 * Math.PI) / numSegments;
        const angle2 = ((i + 1) * 2 * Math.PI) / numSegments;
        segments.push({
          start: { x: cx + radius * Math.cos(angle1), y: cy + radius * Math.sin(angle1) },
          end: { x: cx + radius * Math.cos(angle2), y: cy + radius * Math.sin(angle2) }
        });
      }
      return segments;
    }

    case "diamond": {
      const cx = x + w / 2;
      const cy = y + h / 2;
      return [
        { start: { x: x, y: cy }, end: { x: cx, y: y } }, // Left to top
        { start: { x: cx, y: y }, end: { x: x + w, y: cy } }, // Top to right
        { start: { x: x + w, y: cy }, end: { x: cx, y: y + h } }, // Right to bottom
        { start: { x: cx, y: y + h }, end: { x: x, y: cy } } // Bottom to left
      ];
    }

    case "rectangle":
    default: {
      return [
        { start: { x: x, y: y }, end: { x: x + w, y: y } }, // Top
        { start: { x: x + w, y: y }, end: { x: x + w, y: y + h } }, // Right
        { start: { x: x + w, y: y + h }, end: { x: x, y: y + h } }, // Bottom
        { start: { x: x, y: y + h }, end: { x: x, y: y } } // Left
      ];
    }
  }
}

/** Find closest points between two line segments */
function getClosestPointsBetweenSegments(seg1: LineSegment, seg2: LineSegment): {
  point1: { x: number; y: number };
  point2: { x: number; y: number };
  distance: number;
} {
  const p1 = seg1.start;
  const q1 = seg1.end;
  const p2 = seg2.start;
  const q2 = seg2.end;

  // Vector representations
  const d1x = q1.x - p1.x;
  const d1y = q1.y - p1.y;
  const d2x = q2.x - p2.x;
  const d2y = q2.y - p2.y;
  const r_x = p1.x - p2.x;
  const r_y = p1.y - p2.y;

  // Coefficients for the system of equations
  const a = d1x * d1x + d1y * d1y;
  const b = d1x * d2x + d1y * d2y;
  const c = d2x * d2x + d2y * d2y;
  const d = d1x * r_x + d1y * r_y;
  const e = d2x * r_x + d2y * r_y;

  let s = 0;
  let t = 0;

  // Solve for the closest points
  const denom = a * c - b * b;
  if (Math.abs(denom) > 1e-6) {
    s = Math.max(0, Math.min(1, (b * e - c * d) / denom));
    t = Math.max(0, Math.min(1, (a * e - b * d) / denom));
  } else {
    // Lines are parallel, pick arbitrary points
    s = 0;
    t = Math.max(0, Math.min(1, -e / c));
  }

  // Calculate the closest points
  const point1 = {
    x: p1.x + s * d1x,
    y: p1.y + s * d1y
  };
  const point2 = {
    x: p2.x + t * d2x,
    y: p2.y + t * d2y
  };

  const dx = point2.x - point1.x;
  const dy = point2.y - point1.y;
  const distance = Math.sqrt(dx * dx + dy * dy);

  return { point1, point2, distance };
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

/** Map head style to SVG marker ID (same marker used for both start and end, rotated automatically) */
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
    // Fallback for unknown arrows
    return { markerStart: "", markerEnd: "" };
  }

  const isLeft = entry.is_left;
  const isDiamond = entry.head_style === "diamond_filled" || entry.head_style === "diamond_empty";

  let markerStart = "";
  let markerEnd = "";

  if (isDiamond) {
    // Diamonds are always at the source (from node)
    // For right arrows, diamond is at markerStart
    // For left arrows, diamond is at markerEnd (because direction is reversed)
    if (isLeft) {
      markerEnd = headStyleToMarker(entry.head_style);
    } else {
      markerStart = headStyleToMarker(entry.head_style);
    }
  } else {
    // Regular arrows: head_style goes at the "to" end
    // For left arrows, the visual direction is reversed, so head is at markerStart
    if (isLeft) {
      markerStart = headStyleToMarker(entry.head_style);
    } else {
      markerEnd = headStyleToMarker(entry.head_style);
    }
  }

  return { markerStart, markerEnd };
}
