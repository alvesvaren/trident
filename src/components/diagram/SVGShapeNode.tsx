import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

interface SVGShapeNodeProps {
  node: DiagramNodeType;
  x: number;
  y: number;
  onMouseDown: (e: React.MouseEvent<SVGGElement>) => void;
  onUnlock: (e: React.MouseEvent<SVGGElement>) => void;
  /** Hide interactive elements for export */
  exportMode?: boolean;
}

/** Get shape from modifiers (default: rectangle) */
function getShape(modifiers: string[]): "rectangle" | "circle" | "diamond" {
  if (modifiers.includes("circle")) return "circle";
  if (modifiers.includes("diamond")) return "diamond";
  return "rectangle";
}

/** SVGShapeNode renders node-kind elements (simple shapes with labels) */
export function SVGShapeNode({ node, x, y, onMouseDown, onUnlock, exportMode = false }: SVGShapeNodeProps) {
  const shape = getShape(node.modifiers);
  const label = node.label ?? node.id;
  const w = node.bounds.w;
  const h = node.bounds.h;

  // Center positions
  const cx = w / 2;
  const cy = h / 2;

  // Text styling
  const fontSize = 12;
  const textColor = "var(--canvas-text)";
  const strokeColor = "var(--canvas-border)";
  const fillColor = "var(--canvas-node-bg)";

  return (
    <g transform={`translate(${x}, ${y})`} onMouseDown={onMouseDown} style={{ cursor: exportMode ? "default" : "grab" }}>
      {/* Shape rendering based on type */}
      {shape === "circle" && <ellipse cx={cx} cy={cy} rx={w / 2 - 1} ry={h / 2 - 1} fill={fillColor} stroke={strokeColor} strokeWidth={1} />}

      {shape === "diamond" && <polygon points={`${cx},1 ${w - 1},${cy} ${cx},${h - 1} 1,${cy}`} fill={fillColor} stroke={strokeColor} strokeWidth={1} />}

      {shape === "rectangle" && <rect x={0} y={0} width={w} height={h} rx={4} ry={4} fill={fillColor} stroke={strokeColor} strokeWidth={1} />}

      {/* Label centered in shape */}
      <text x={cx} y={cy} textAnchor='middle' dominantBaseline='central' fill={textColor} fontSize={fontSize} fontFamily='ui-monospace, monospace'>
        {label}
      </text>

      {/* Lock icon for fixed position */}
      {node.has_pos && !exportMode && (
        <g
          transform={`translate(${w - 16}, 4)`}
          onClick={e => {
            e.stopPropagation();
            onUnlock(e as unknown as React.MouseEvent<SVGGElement>);
          }}
          style={{ cursor: "pointer" }}
        >
          <rect x={-2} y={-2} width={16} height={16} fill='transparent' />
          <svg width={12} height={12} viewBox='0 0 24 24'>
            <rect x='3' y='11' width='18' height='11' rx='2' fill='none' stroke='var(--canvas-text-muted)' strokeWidth='2' />
            <path d='M7 11V7a5 5 0 0110 0v4' fill='none' stroke='var(--canvas-text-muted)' strokeWidth='2' strokeLinecap='round' />
          </svg>
        </g>
      )}
    </g>
  );
}
