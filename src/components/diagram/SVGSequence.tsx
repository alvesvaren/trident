import type { DiagramEdge, DiagramNode } from "../../types/diagram";

interface SVGSequenceEdgesProps {
  edges: DiagramEdge[];
  nodes: DiagramNode[];
}

export function SVGSequenceEdges({ edges, nodes }: SVGSequenceEdgesProps) {
  const nodeMap = new Map(nodes.map(n => [n.id, n]));

  return (
    <g className="sequence-edges">
      {edges.map((edge, i) => {
        const fromNode = nodeMap.get(edge.from);
        const toNode = nodeMap.get(edge.to);
        if (!fromNode || !toNode || edge.y === undefined) return null;

        const fromX = fromNode.bounds.x + fromNode.bounds.w / 2;
        const toX = toNode.bounds.x + toNode.bounds.w / 2;
        const y = edge.y;
        const isDashed = edge.arrow === "seq_return";

        const labelX = (fromX + toX) / 2;

        return (
          <g key={i}>
            <line
              x1={fromX}
              y1={y}
              x2={toX}
              y2={y}
              stroke="var(--canvas-edge)"
              strokeWidth={1.5}
              strokeDasharray={isDashed ? "8,4" : undefined}
              markerEnd="url(#arrowhead)"
            />
            {edge.label && (
              <g>
                <rect
                  x={labelX - edge.label.length * 3.3}
                  y={y - 16}
                  width={edge.label.length * 6.6}
                  height={14}
                  rx={2}
                  ry={2}
                  fill="var(--canvas-bg)"
                />
                <text
                  x={labelX}
                  y={y - 8}
                  fill="var(--canvas-text)"
                  fontSize={11}
                  fontFamily="ui-monospace, monospace"
                  textAnchor="middle"
                >
                  {edge.label}
                </text>
              </g>
            )}
          </g>
        );
      })}
    </g>
  );
}

interface SVGLifelinesProps {
  lifelines: NonNullable<import("../../types/diagram").DiagramOutput["lifelines"]>;
}

export function SVGLifelines({ lifelines }: SVGLifelinesProps) {
  return (
    <g className="lifelines">
      {lifelines.map((line, i) => (
        <line
          key={i}
          x1={line.x}
          y1={line.y1}
          x2={line.x}
          y2={line.y2}
          stroke="var(--canvas-border)"
          strokeWidth={1}
          strokeDasharray="6,4"
        />
      ))}
    </g>
  );
}

interface SVGActivationsProps {
  activations: NonNullable<import("../../types/diagram").DiagramOutput["activations"]>;
}

export function SVGActivations({ activations }: SVGActivationsProps) {
  return (
    <g className="activations">
      {activations.map((act, i) => (
        <rect
          key={i}
          x={act.x}
          y={act.y}
          width={14}
          height={act.h}
          fill="var(--canvas-node-bg)"
          stroke="var(--canvas-edge)"
          strokeWidth={1}
        />
      ))}
    </g>
  );
}
