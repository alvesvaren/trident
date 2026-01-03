import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

interface SVGNodeProps {
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

// Color mappings (Tailwind -> hex)
const BORDER_COLORS: Record<string, string> = {
  interface: "#22c55e",
  enum: "#a855f7",
  struct: "#f97316",
  record: "#f97316",
  trait: "#06b6d4",
  object: "#eab308",
  class: "#525252",
};

const TITLE_COLORS: Record<string, string> = {
  interface: "#86efac",
  enum: "#d8b4fe",
  struct: "#fdba74",
  record: "#fdba74",
  trait: "#67e8f9",
  object: "#fde047",
  class: "#93c5fd",
};

/** Format node kind for display (e.g., "interface" -> "«interface»") */
function formatKindStereotype(kind: string): string | null {
  if (kind === "class") return null;
  return `«${kind}»`;
}

/** Format modifiers for display (e.g., ["abstract", "sealed"] -> "«abstract» «sealed»") */
function formatModifiers(modifiers: string[]): string | null {
  if (modifiers.length === 0) return null;
  return modifiers.map(m => `«${m}»`).join(" ");
}

export function SVGNode({ node, x, y, onMouseDown, onUnlock, onResizeStart, exportMode = false }: SVGNodeProps) {
  const kindStereotype = formatKindStereotype(node.kind);
  const modifierStereotypes = formatModifiers(node.modifiers);
  const borderColor = BORDER_COLORS[node.kind] ?? BORDER_COLORS.class;
  const titleColor = TITLE_COLORS[node.kind] ?? TITLE_COLORS.class;

  const allStereotypes = [modifierStereotypes, kindStereotype].filter(Boolean).join(" ");
  const isAbstract = node.modifiers.includes("abstract");
  const w = node.bounds.w;
  const h = node.bounds.h;

  // Center for handles
  const cx = w / 2;
  const cy = h / 2;

  // Layout constants
  const padding = 8;
  const lineHeight = 14;
  const titleFontSize = 12;
  const stereotypeFontSize = 10;
  const bodyFontSize = 11;
  const separatorY = padding + (allStereotypes ? lineHeight : 0) + lineHeight + 4;

  // Calculate y positions for text elements
  let currentY = padding + (allStereotypes ? stereotypeFontSize : 0);
  const stereotypeY = padding + stereotypeFontSize - 2;
  const titleY = currentY + titleFontSize;
  currentY = separatorY + 4;

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
      {/* Background */}
      <rect x={0} y={0} width={node.bounds.w} height={node.bounds.h} rx={4} ry={4} fill='var(--canvas-node-bg)' stroke={borderColor} strokeWidth={1} />

      {/* Stereotype line */}
      {allStereotypes && (
        <text
          x={node.bounds.w / 2}
          y={stereotypeY}
          textAnchor='middle'
          fill='var(--canvas-text)'
          fontSize={stereotypeFontSize}
          fontFamily='ui-monospace, monospace'
          fontStyle='italic'
        >
          {allStereotypes}
        </text>
      )}

      {/* Title */}
      <text
        x={padding}
        y={titleY}
        fill={titleColor}
        fontSize={titleFontSize}
        fontFamily='ui-monospace, monospace'
        fontWeight='bold'
        fontStyle={isAbstract ? "italic" : "normal"}
      >
        {node.label ?? node.id}
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

      {/* Lock icon */}
      {node.has_pos && !exportMode && (
        <g
          transform={`translate(${node.bounds.w - padding - 12}, ${titleY - 10})`}
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

      {/* Separator line */}
      <line x1={0} y1={separatorY} x2={node.bounds.w} y2={separatorY} stroke='var(--canvas-border)' strokeWidth={1} />

      {/* Body lines */}
      {node.body_lines.map((line, i) => (
        <text
          key={i}
          x={padding}
          y={separatorY + 4 + (i + 1) * lineHeight}
          fill='var(--canvas-text)'
          fontSize={bodyFontSize}
          fontFamily='ui-monospace, monospace'
        >
          {line}
        </text>
      ))}
    </g>
  );
}
