import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

interface SVGParticipantProps {
  node: DiagramNodeType;
  x: number;
  y: number;
}

export function SVGParticipant({ node, x, y }: SVGParticipantProps) {
  const w = node.bounds.w;
  const h = node.bounds.h;
  const title = node.text_elements.find(el => el.type === "Title");

  return (
    <g transform={`translate(${x}, ${y})`}>
      <rect
        x={0}
        y={0}
        width={w}
        height={h}
        rx={4}
        ry={4}
        fill="var(--canvas-node-bg)"
        stroke="var(--canvas-border)"
        strokeWidth={1}
      />
      {title && (
        <text
          x={w / 2}
          y={title.data.y}
          textAnchor="middle"
          fill="var(--canvas-text)"
          fontSize={title.data.font_size}
          fontFamily="ui-monospace, monospace"
          fontWeight="bold"
        >
          {title.data.text}
        </text>
      )}
    </g>
  );
}
