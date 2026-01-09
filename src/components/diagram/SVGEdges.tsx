/**
 * SVG Edge components for diagram rendering
 * Extracted from EdgeOverlay to work in a unified SVG context
 */

import type { DiagramEdge, DiagramNode, DragState, Bounds } from "../../types/diagram";
import { getCenter, getEdgePoint, getEdgeMarkers, isDashed, getShape, getOptimalConnectionPoints } from "../../utils/geometry";
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

        const midX = (start.x + end.x) / 2;
        const midY = (start.y + end.y) / 2;

        return (
          <g key={i}>
            <line
              x1={start.x}
              y1={start.y}
              x2={end.x}
              y2={end.y}
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
