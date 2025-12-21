import type { Bounds } from "../types/diagram";

/** Get center of a bounds rectangle */
export function getCenter(b: Bounds): { x: number; y: number } {
    return { x: b.x + b.w / 2, y: b.y + b.h / 2 };
}

/** Calculate intersection point of line from center to target with rectangle edge */
export function getEdgePoint(
    bounds: Bounds,
    targetX: number,
    targetY: number
): { x: number; y: number } {
    const cx = bounds.x + bounds.w / 2;
    const cy = bounds.y + bounds.h / 2;
    const dx = targetX - cx;
    const dy = targetY - cy;

    if (dx === 0 && dy === 0) return { x: cx, y: cy };

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

