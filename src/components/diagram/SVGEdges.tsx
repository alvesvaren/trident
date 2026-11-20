/**
 * SVG Edge components for diagram rendering
 * Extracted from EdgeOverlay to work in a unified SVG context
 */

import type { DiagramEdge, DiagramNode, DragState, Bounds } from "../../types/diagram";
import { getEdgeMarkers, isDashed, getShape, getOptimalConnectionPoints } from "../../utils/geometry";
/**
 * SVG marker definitions for edge arrows
 * These should be placed in the <defs> section of the parent SVG
 *
 * We use a single marker definition per type with orient='auto-start-reverse':
 * - When used as markerEnd: points forward along the line (normal direction)
 * - When used as markerStart: automatically reverses to point backward (towards start)
 * - refX is set to position the marker correctly at the line endpoint
 */
export function EdgeDefs() {
  return (
    <>
      <marker id='arrowhead' markerWidth='8' markerHeight='8' refX='7' refY='4' orient='auto-start-reverse'>
        <polyline points='1 1, 7 4, 1 7' fill='var(--canvas-marker-fill)' stroke='var(--canvas-edge)' strokeWidth='1' />
        <line x1='0' y1='4' x2='7' y2='4' stroke='var(--canvas-edge)' strokeWidth='1' />
      </marker>

      <marker id='rounded-arrowhead' markerWidth='8' markerHeight='8' refX='3' refY='4' orient='auto-start-reverse'>
        <path d='M 1 1 Q 5 4, 1 7' fill='none' stroke='var(--canvas-edge)' strokeWidth='1' />
      </marker>

      <marker id='triangle' markerWidth='10' markerHeight='9' refX='9' refY='4.5' orient='auto-start-reverse'>
        <polygon points='1 1, 9 4.5, 1 8' fill='var(--canvas-marker-fill)' stroke='var(--canvas-edge)' strokeWidth='1' />
      </marker>

      <marker id='diamond' markerWidth='13' markerHeight='10' refX='11' refY='5' orient='auto-start-reverse'>
        <polygon points='0 5, 6 1, 12 5, 6 9' fill='var(--canvas-edge)' />
      </marker>

      <marker id='diamond-empty' markerWidth='13' markerHeight='10' refX='11' refY='5' orient='auto-start-reverse'>
        <polygon points='0 5, 6 1, 12 5, 6 9' fill='var(--canvas-marker-fill)' stroke='var(--canvas-edge)' strokeWidth='1' />
      </marker>
    </>
  );
}

/** Distance between the apexes of neighbouring parallel edges */
const PARALLEL_EDGE_SPREAD = 28;

/**
 * For edges sharing the same pair of nodes, how far each one should bow out sideways
 * (0 for edges without a parallel twin), so they don't draw on top of each other.
 */
function parallelEdgeBends(edges: DiagramEdge[]): number[] {
  const pairKey = (e: DiagramEdge) => [e.from, e.to].sort().join("\0");
  const groups = new Map<string, number[]>();
  edges.forEach((e, i) => groups.set(pairKey(e), [...(groups.get(pairKey(e)) ?? []), i]));

  const bends = edges.map(() => 0);
  for (const indices of groups.values()) {
    indices.forEach((edgeIndex, k) => {
      bends[edgeIndex] = (k - (indices.length - 1) / 2) * PARALLEL_EDGE_SPREAD;
    });
  }
  return bends;
}

interface SVGEdgesProps {
  edges: DiagramEdge[];
  nodes: DiagramNode[];
  dragState: DragState | null;
}

/**
 * Render all edges as SVG elements
 * This should be placed inside the parent SVG, not as a wrapper
 */
export function SVGEdges({ edges, nodes, dragState }: SVGEdgesProps) {
  // Build a map from node id to bounds
  const nodeMap = new Map<string, Bounds>();
  nodes.forEach(n => nodeMap.set(n.id, n.bounds));
  const bends = parallelEdgeBends(edges);

  return (
    <g className='edges'>
      {edges.map((edge, i) => {
        const fromNode = nodes.find(n => n.id === edge.from);
        const toNode = nodes.find(n => n.id === edge.to);

        let fromBounds = nodeMap.get(edge.from);
        let toBounds = nodeMap.get(edge.to);

        // Update bounds if node is being dragged
        if (dragState?.type === "node" && fromNode && dragState.id === fromNode.id) {
          fromBounds = { ...fromNode.bounds, x: dragState.currentX, y: dragState.currentY };
        }
        if (dragState?.type === "node" && toNode && dragState.id === toNode.id) {
          toBounds = { ...toNode.bounds, x: dragState.currentX, y: dragState.currentY };
        }

        if (!fromBounds || !toBounds) return null;

        const fromShape = fromNode ? getShape(fromNode.modifiers) : "rectangle";
        const toShape = toNode ? getShape(toNode.modifiers) : "rectangle";

        const { markerStart, markerEnd } = getEdgeMarkers(edge.arrow);

        // Apply offset only at the arrow-head end (where the marker is)
        const ARROW_OFFSET = 5;
        const startOffset = markerStart ? ARROW_OFFSET : 0;
        const endOffset = markerEnd ? ARROW_OFFSET : 0;

        // Use optimal connection points for shortest arrows
        const { start, end } = getOptimalConnectionPoints(
          fromBounds,
          toBounds,
          fromShape,
          toShape,
          startOffset,
          endOffset
        );

        // Parallel edges bow out sideways. The normal is taken in a direction-independent
        // order (by node id) so A->B and B->A between the same nodes bend apart too.
        const [ax, ay, bx, by] = edge.from < edge.to ? [start.x, start.y, end.x, end.y] : [end.x, end.y, start.x, start.y];
        const len = Math.hypot(bx - ax, by - ay) || 1;
        const normal = { x: -(by - ay) / len, y: (bx - ax) / len };
        const bend = bends[i];
        const midX = (start.x + end.x) / 2 + normal.x * bend;
        const midY = (start.y + end.y) / 2 + normal.y * bend;
        // A quadratic curve's apex lies halfway to its control point
        const controlX = (start.x + end.x) / 2 + normal.x * bend * 2;
        const controlY = (start.y + end.y) / 2 + normal.y * bend * 2;

        return (
          <g key={i}>
            <path
              d={bend === 0 ? `M ${start.x} ${start.y} L ${end.x} ${end.y}` : `M ${start.x} ${start.y} Q ${controlX} ${controlY} ${end.x} ${end.y}`}
              fill='none'
              stroke='var(--canvas-edge)'
              strokeWidth={1.5}
              strokeDasharray={isDashed(edge.arrow) ? "8,4" : undefined}
              markerEnd={markerEnd}
              markerStart={markerStart}
            />
            {edge.label && (() => {
              const fontSize = 11;
              // Estimate text width for monospace font (roughly 0.6 * fontSize per character)
              const estimatedTextWidth = edge.label.length * fontSize * 0.6;
              const rectWidth = estimatedTextWidth;
              const rectHeight = fontSize;
              
              return (
                <g>
                  <rect
                    x={midX - rectWidth / 2}
                    y={midY - rectHeight / 2}
                    width={rectWidth}
                    height={rectHeight}
                    rx={2}
                    ry={2}
                    fill='var(--canvas-bg)'
                  />
                  <text
                    x={midX}
                    y={midY}
                    fill='var(--canvas-text)'
                    fontSize={fontSize}
                    fontFamily='ui-monospace, monospace'
                    textAnchor='middle'
                    dominantBaseline='central'
                  >
                    {edge.label}
                  </text>
                </g>
              );
            })()}
          </g>
        );
      })}
    </g>
  );
}
