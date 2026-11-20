import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

import { getShape, hasDoubleBorder, trianglePoints, type NodeShape } from "../../utils/geometry";
import { StyledText } from "./StyledText";

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

const DOUBLE_GAP = 4;

/** Outline of a shape filling a w×h box, shrunk by `inset` on every side */
function ShapeOutline({ shape, w, h, inset, fill, stroke }: { shape: NodeShape; w: number; h: number; inset: number; fill: string; stroke: string }) {
  const i = inset + 1; // keep the 1px stroke inside the box
  switch (shape) {
    case "circle":
      return <ellipse cx={w / 2} cy={h / 2} rx={w / 2 - i} ry={h / 2 - i} fill={fill} stroke={stroke} strokeWidth={1} />;
    case "diamond": {
      // Inset along the slanted edges: scale the half-diagonals so the edge moves `inset` inwards
      const k = Math.hypot(w / 2, h / 2);
      const ix = (i * k) / (h / 2);
      const iy = (i * k) / (w / 2);
      const points = `${w / 2},${iy} ${w - ix},${h / 2} ${w / 2},${h - iy} ${ix},${h / 2}`;
      return <polygon points={points} fill={fill} stroke={stroke} strokeWidth={1} />;
    }
    case "triangle":
      return <polygon points={trianglePoints(w, h, i).map(p => p.join(",")).join(" ")} fill={fill} stroke={stroke} strokeWidth={1} />;
    case "rectangle":
      return <rect x={inset} y={inset} width={w - 2 * inset} height={h - 2 * inset} rx={4} ry={4} fill={fill} stroke={stroke} strokeWidth={1} />;
  }
}

/** SVGShapeNode renders node-kind elements (simple shapes with labels) */
export function SVGShapeNode({ node, x, y, onMouseDown, onUnlock, onResizeStart, exportMode = false }: SVGShapeNodeProps) {
  const shape = getShape(node.modifiers);
  const title = node.text_elements.find(el => el.type === "Title");
  const labelSpans =
    title?.type === "Title" ? title.data.spans : [{ text: node.label ?? node.id, italic: false, underline: false, dashed_underline: false }];
  const double = hasDoubleBorder(node.modifiers);
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
      {/* Shape rendering based on type; double-bordered nodes get a second outline inset by DOUBLE_GAP */}
      {[0, ...(double ? [DOUBLE_GAP] : [])].map(inset => (
        <ShapeOutline key={inset} shape={shape} w={w} h={h} inset={inset} fill={inset === 0 ? fillColor : "none"} stroke={strokeColor} />
      ))}

      {/* Label centered in shape (lower part for triangles, where there's room) */}
      <text
        x={cx}
        y={shape === "triangle" ? h * 0.68 : cy}
        textAnchor='middle'
        dominantBaseline='central'
        fill={textColor}
        fontSize={fontSize}
        fontFamily='ui-monospace, monospace'
      >
        <StyledText spans={labelSpans} />
      </text>

      {/* Resize Handles */}
      {!exportMode && onResizeStart && (
        <>
          {/* Invisible sensor for edge detection */}
          <rect
            className='edge-sensor'
            x={0}
            y={0}
            width={w}
            height={h}
            fill='none'
            stroke='transparent'
            strokeWidth={20}
            style={{ pointerEvents: "stroke" }}
          />

          <g className='resize-handles' style={{ opacity: 0, transition: "opacity 0.2s" }}>
            <style>{`
                .edge-sensor:hover ~ .resize-handles,
                .resize-handles:hover { opacity: 1 !important; }
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
        </>
      )}

      {/* Lock icon for fixed position */}
      {node.has_pos && !exportMode && (
        <g
          transform={`translate(${w - 16}, 4)`}
          onMouseDown={e => e.stopPropagation()}
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
