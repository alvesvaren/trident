/**
 * SVG Edge components for diagram rendering
 * Extracted from EdgeOverlay to work in a unified SVG context
 */

import type { DiagramEdge, DiagramNode, DragState, Bounds } from "../../types/diagram";
import { getCenter, getEdgePoint, getEdgeMarkers, isDashed } from "../../utils/geometry";

/**
 * SVG marker definitions for edge arrows
 * These should be placed in the <defs> section of the parent SVG
 * 
 * We have separate markers for start and end positions:
 * - *-end markers have refX at the tip so the line stops at the back of the arrow
 * - *-start markers have refX at 0 so the shape appears at the line start (node border)
 */
export function EdgeDefs() {
    return (
        <>
            {/* Arrowhead for markerEnd - refX at tip so line ends at back of arrow */}
            <marker
                id="arrowhead-end"
                markerWidth="10"
                markerHeight="7"
                refX="10"
                refY="3.5"
                orient="auto"
            >
                <polygon points="0 0, 10 3.5, 0 7" fill="#888" />
            </marker>
            {/* Arrowhead for markerStart - refX at 0 so arrow appears at line start */}
            <marker
                id="arrowhead-start"
                markerWidth="10"
                markerHeight="7"
                refX="0"
                refY="3.5"
                orient="auto"
            >
                <polygon points="0 3.5, 10 0, 10 7" fill="#888" />
            </marker>

            {/* Triangle (extends) for markerEnd - hollow triangle, line stops at back */}
            <marker
                id="triangle-end"
                markerWidth="12"
                markerHeight="10"
                refX="12"
                refY="5"
                orient="auto"
            >
                <polygon
                    points="0 0, 12 5, 0 10"
                    fill="#1e1e1e"
                    stroke="#888"
                    strokeWidth="1.5"
                />
            </marker>
            {/* Triangle (extends) for markerStart - hollow triangle at line start */}
            <marker
                id="triangle-start"
                markerWidth="12"
                markerHeight="10"
                refX="0"
                refY="5"
                orient="auto"
            >
                <polygon
                    points="0 5, 12 0, 12 10"
                    fill="#1e1e1e"
                    stroke="#888"
                    strokeWidth="1.5"
                />
            </marker>

            {/* Filled diamond (composition) for markerStart - appears at source node */}
            <marker
                id="diamond-start"
                markerWidth="12"
                markerHeight="8"
                refX="0"
                refY="4"
                orient="auto"
            >
                <polygon points="0 4, 6 0, 12 4, 6 8" fill="#888" />
            </marker>

            {/* Empty diamond (aggregation) for markerStart - appears at source node */}
            <marker
                id="diamond-empty-start"
                markerWidth="12"
                markerHeight="8"
                refX="0"
                refY="4"
                orient="auto"
            >
                <polygon
                    points="0 4, 6 0, 12 4, 6 8"
                    fill="#1e1e1e"
                    stroke="#888"
                    strokeWidth="1"
                />
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
    nodes.forEach((n) => nodeMap.set(n.id, n.bounds));

    return (
        <g className="edges">
            {edges.map((edge, i) => {
                const fromNode = nodes.find((n) => n.id === edge.from);
                const toNode = nodes.find((n) => n.id === edge.to);

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

                const fromCenter = getCenter(fromBounds);
                const toCenter = getCenter(toBounds);
                const start = getEdgePoint(fromBounds, toCenter.x, toCenter.y);
                const end = getEdgePoint(toBounds, fromCenter.x, fromCenter.y);
                const { markerStart, markerEnd } = getEdgeMarkers(edge.arrow);

                const midX = (start.x + end.x) / 2;
                const midY = (start.y + end.y) / 2;

                return (
                    <g key={i}>
                        <line
                            x1={start.x}
                            y1={start.y}
                            x2={end.x}
                            y2={end.y}
                            stroke="#888"
                            strokeWidth={1.5}
                            strokeDasharray={isDashed(edge.arrow) ? "5,3" : undefined}
                            markerEnd={markerEnd}
                            markerStart={markerStart}
                        />
                        {edge.label && (
                            <text
                                x={midX}
                                y={midY - 6}
                                fill="#aaa"
                                fontSize={11}
                                fontFamily="ui-monospace, monospace"
                                textAnchor="middle"
                            >
                                {edge.label}
                            </text>
                        )}
                    </g>
                );
            })}
        </g>
    );
}
