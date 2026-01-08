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


export function SVGNode({ node, x, y, onMouseDown, onUnlock, onResizeStart, exportMode = false }: SVGNodeProps) {
  const borderColor = BORDER_COLORS[node.kind] ?? BORDER_COLORS.class;
  const titleColor = TITLE_COLORS[node.kind] ?? TITLE_COLORS.class;

  const w = node.bounds.w;
  const h = node.bounds.h;

  // Center for handles
  const cx = w / 2;
  const cy = h / 2;

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
      {node.has_pos && !exportMode && (() => {
        // Find the title element to position the lock icon
        const titleElement = node.text_elements.find(el => el.type === "Title");
        const titleY = titleElement ? titleElement.data.y : node.rendering_config.padding + 12;
        return (
          <g
            transform={`translate(${node.bounds.w - node.rendering_config.padding - 12}, ${titleY - 10})`}
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
        );
      })()}

      {/* Render all text elements */}
      {node.text_elements.map((element, i) => {
        switch (element.type) {
          case "Stereotype":
            return (
              <text
                key={i}
                x={node.bounds.w / 2}
                y={element.data.y}
                textAnchor='middle'
                fill='var(--canvas-text)'
                fontSize={element.data.font_size}
                fontFamily='ui-monospace, monospace'
                fontStyle='italic'
              >
                {element.data.text}
              </text>
            );
          case "Title":
            return (
              <text
                key={i}
                x={node.rendering_config.padding}
                y={element.data.y}
                fill={titleColor}
                fontSize={element.data.font_size}
                fontFamily='ui-monospace, monospace'
                fontWeight='bold'
                fontStyle={element.data.italic ? "italic" : "normal"}
              >
                {element.data.text}
              </text>
            );
          case "Separator":
            return (
              <line
                key={i}
                x1={0}
                y1={element.data.y1}
                x2={node.bounds.w}
                y2={element.data.y2}
                stroke='var(--canvas-border)'
                strokeWidth={1}
              />
            );
          case "BodyText":
            return (
              <text
                key={i}
                x={node.rendering_config.padding}
                y={element.data.y}
                fill='var(--canvas-text)'
                fontSize={element.data.font_size}
                fontFamily='ui-monospace, monospace'
              >
                {element.data.text}
              </text>
            );
        }
      })}
    </g>
  );
}
