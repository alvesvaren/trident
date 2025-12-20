import { useMemo } from "react";
import type { DiagramEdge, DiagramNode, DragState, Bounds } from "../../types/diagram";
import { getCenter, getEdgePoint, getEdgeMarkers, isDashed } from "../../utils/geometry";

interface EdgeOverlayProps {
    edges: DiagramEdge[];
    nodes: DiagramNode[];
    dragState: DragState | null;
}

export function EdgeOverlay({ edges, nodes, dragState }: EdgeOverlayProps) {
    // Build a map from node id to bounds
    const nodeMap = useMemo(() => {
        const map = new Map<string, Bounds>();
        nodes.forEach((n) => map.set(n.id, n.bounds));
        return map;
    }, [nodes]);

    // Calculate SVG viewport size based on all nodes
    const svgSize = useMemo(() => {
        let maxX = 0;
        let maxY = 0;
        nodes.forEach((n) => {
            maxX = Math.max(maxX, n.bounds.x + n.bounds.w);
            maxY = Math.max(maxY, n.bounds.y + n.bounds.h);
        });
        return { width: maxX + 50, height: maxY + 50 };
    }, [nodes]);

    return (
        <svg
            style={{
                position: "absolute",
                top: 0,
                left: 0,
                pointerEvents: "none",
                zIndex: 10,
            }}
            width={svgSize.width}
            height={svgSize.height}
        >
            <defs>
                <marker
                    id="arrowhead"
                    markerWidth="10"
                    markerHeight="7"
                    refX="9"
                    refY="3.5"
                    orient="auto"
                >
                    <polygon points="0 0, 10 3.5, 0 7" fill="#888" />
                </marker>
                <marker
                    id="triangle"
                    markerWidth="12"
                    markerHeight="10"
                    refX="11"
                    refY="5"
                    orient="auto"
                >
                    <polygon
                        points="0 0, 12 5, 0 10"
                        fill="none"
                        stroke="#888"
                        strokeWidth="1"
                    />
                </marker>
                <marker
                    id="diamond"
                    markerWidth="12"
                    markerHeight="8"
                    refX="11"
                    refY="4"
                    orient="auto"
                >
                    <polygon points="0 4, 6 0, 12 4, 6 8" fill="#888" />
                </marker>
                <marker
                    id="diamond-empty"
                    markerWidth="12"
                    markerHeight="8"
                    refX="11"
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
            </defs>

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
                                fontFamily="Fira Code VF"
                                textAnchor="middle"
                            >
                                {edge.label}
                            </text>
                        )}
                    </g>
                );
            })}
        </svg>
    );
}
