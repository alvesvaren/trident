import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

import { getShape } from "../../utils/geometry";

interface SVGShapeNodeProps {
  node: DiagramNodeType;
  x: number;
  y: number;
  onMouseDown: (e: React.MouseEvent<SVGGElement>) => void;
  onUnlock: (e: React.MouseEvent<SVGGElement>) => void;
  /** Callback for resize start */
  onResizeStart?: (e: React.MouseEvent, node: DiagramNodeType, handle: "nw" | "ne" | "sw" | "se" | "n" | "e" | "s" | "w") => void;
  /** Hide interactive elements for export */
  exportMode?: boolean;
}

/** SVGShapeNode renders node-kind elements (simple shapes with labels) */
export function SVGShapeNode({ node, x, y, onMouseDown, onUnlock, onResizeStart, exportMode = false }: SVGShapeNodeProps) {
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

  // Resize handle styling
  const handleSize = 8;
  const handleStyle: React.CSSProperties = {
    fill: "var(--accent)",
    stroke: "var(--canvas-bg)",
    strokeWidth: 1,
    cursor: "pointer",
  };

  // Helper to create a resize handle
  const ResizeHandle = ({ cx, cy, cursor, handle }: { cx: number; cy: number; cursor: string; handle: "nw" | "ne" | "sw" | "se" | "n" | "e" | "s" | "w" }) => (
    <rect
      x={cx - handleSize / 2}
      y={cy - handleSize / 2}
      width={handleSize}
      height={handleSize}
      style={{ ...handleStyle, cursor }}
      onMouseDown={e => {
        e.stopPropagation();
        onResizeStart?.(e, node, handle);
      }}
    />
  );

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

      {/* Resize Handles */}
      {!exportMode && onResizeStart && (
        <g className='resize-handles' style={{ opacity: 0, transition: "opacity 0.2s" }}>
          {/* We use specific class on parent group to show handles on hover? 
              Actually, React doesn't support parent hover easily. 
              Let's make them always visible but subtle, or rely on CSS .node:hover .resize-handles (if we had access to parent).
              For now, let's just render them. 
              Ideally we'd use a separate logic to only show them on selection/hover.
              Let's render them consistently for now as 'subtle' or rely on standard UI patterns.
              Wait, better yet: SVG doesn't support hover classes easily without CSS.
              Let's just render them.
          */}
          <style>{`
            g:hover > .resize-handles { opacity: 1 !important; }
          `}</style>

          {/* Corners */}
          <ResizeHandle cx={0} cy={0} cursor='nw-resize' handle='nw' />
          <ResizeHandle cx={w} cy={0} cursor='ne-resize' handle='ne' />
          <ResizeHandle cx={w} cy={h} cursor='se-resize' handle='se' />
          <ResizeHandle cx={0} cy={h} cursor='sw-resize' handle='sw' />

          {/* Edges */}
          <ResizeHandle cx={cx} cy={0} cursor='n-resize' handle='n' />
          <ResizeHandle cx={w} cy={cy} cursor='e-resize' handle='e' />
          <ResizeHandle cx={cx} cy={h} cursor='s-resize' handle='s' />
          <ResizeHandle cx={0} cy={cy} cursor='w-resize' handle='w' />
        </g>
      )}

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
