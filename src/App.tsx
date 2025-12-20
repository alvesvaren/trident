import { useEffect, useMemo, useState } from "react";
import { compile_diagram } from "trident-core";

import Editor from "@monaco-editor/react";
import { registerSddLanguage } from "./syntax";

interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface DiagramNode {
  id: string;
  label: string | null;
  body_lines: string[];
  bounds: Bounds;
}

interface DiagramEdge {
  from: string;
  to: string;
  arrow: string;
  label: string | null;
}

interface DiagramGroup {
  id: string;
  bounds: Bounds;
}

interface DiagramOutput {
  groups?: DiagramGroup[];
  nodes?: DiagramNode[];
  edges?: DiagramEdge[];
  error?: string;
}

/** Get center of a bounds rectangle */
function getCenter(b: Bounds): { x: number; y: number } {
  return { x: b.x + b.w / 2, y: b.y + b.h / 2 };
}

/** Calculate intersection point of line from center to target with rectangle edge */
function getEdgePoint(
  bounds: Bounds,
  targetX: number,
  targetY: number
): { x: number; y: number } {
  const cx = bounds.x + bounds.w / 2;
  const cy = bounds.y + bounds.h / 2;
  const dx = targetX - cx;
  const dy = targetY - cy;

  if (dx === 0 && dy === 0) return { x: cx, y: cy };

  const halfW = bounds.w / 2;
  const halfH = bounds.h / 2;

  // Calculate intersection with each edge
  const tRight = halfW / Math.abs(dx);
  const tLeft = halfW / Math.abs(dx);
  const tBottom = halfH / Math.abs(dy);
  const tTop = halfH / Math.abs(dy);

  let t = Infinity;

  if (dx > 0) t = Math.min(t, tRight);
  if (dx < 0) t = Math.min(t, tLeft);
  if (dy > 0) t = Math.min(t, tBottom);
  if (dy < 0) t = Math.min(t, tTop);

  return {
    x: cx + dx * t,
    y: cy + dy * t,
  };
}

/** Check if arrow points to the "from" node (left arrows) */
function isLeftArrow(arrow: string): boolean {
  return arrow.endsWith("_left");
}

/** Check if the edge should be dashed */
function isDashed(arrow: string): boolean {
  return arrow === "dotted" || arrow.startsWith("dep_");
}

function App() {
  const [code, setCode] = useState("");
  const [result, setResult] = useState<DiagramOutput>({});

  useEffect(() => {
    const start = performance.now();
    const jsonResult = compile_diagram(code);
    setResult(JSON.parse(jsonResult));
    const end = performance.now();
    console.log(`Time taken to parse: ${end - start} milliseconds`);
  }, [code]);

  // Build a map from node id to bounds for edge rendering
  const nodeMap = useMemo(() => {
    const map = new Map<string, Bounds>();
    result.nodes?.forEach((n) => map.set(n.id, n.bounds));
    return map;
  }, [result.nodes]);

  // Calculate SVG viewport size based on all nodes
  const svgSize = useMemo(() => {
    let maxX = 0,
      maxY = 0;
    result.nodes?.forEach((n) => {
      maxX = Math.max(maxX, n.bounds.x + n.bounds.w);
      maxY = Math.max(maxY, n.bounds.y + n.bounds.h);
    });
    return { width: maxX + 50, height: maxY + 50 };
  }, [result.nodes]);

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      <Editor
        beforeMount={registerSddLanguage}
        language="trident"
        theme="trident-dark"
        height="100vh"
        width="50vw"
        value={code}
        options={{
          minimap: { enabled: false },
          fontLigatures: false,
          fontFamily: "Fira Code VF",
        }}
        onChange={(value) => setCode(value ?? "")}
      />
      <div
        id="diagram"
        style={{
          flex: 1,
          position: "relative",
          overflow: "auto",
          backgroundColor: "#1e1e1e",
        }}
      >
        {result.error && (
          <div style={{ color: "#f44", padding: 16 }}>{result.error}</div>
        )}

        {/* SVG layer for edges - render on top of groups */}
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
            {/* Arrow marker for lines */}
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
            {/* Triangle marker for inheritance */}
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
            {/* Diamond marker for aggregate/compose */}
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

          {result.edges?.map((edge, i) => {
            const fromBounds = nodeMap.get(edge.from);
            const toBounds = nodeMap.get(edge.to);
            if (!fromBounds || !toBounds) return null;

            const fromCenter = getCenter(fromBounds);
            const toCenter = getCenter(toBounds);

            // Determine which end gets the arrow
            const leftArrow = isLeftArrow(edge.arrow);
            const arrowAtFrom = leftArrow;

            // Calculate edge points at rectangle boundaries
            const start = getEdgePoint(fromBounds, toCenter.x, toCenter.y);
            const end = getEdgePoint(toBounds, fromCenter.x, fromCenter.y);

            // Choose marker based on arrow type
            let markerEnd = "";
            let markerStart = "";
            const baseArrow = edge.arrow.replace("_left", "").replace("_right", "");

            if (baseArrow === "extends") {
              if (arrowAtFrom) markerStart = "url(#triangle)";
              else markerEnd = "url(#triangle)";
            } else if (baseArrow === "assoc" || baseArrow === "dep") {
              if (arrowAtFrom) markerStart = "url(#arrowhead)";
              else markerEnd = "url(#arrowhead)";
            } else if (baseArrow === "aggregate") {
              markerStart = "url(#diamond-empty)";
            } else if (baseArrow === "compose") {
              markerStart = "url(#diamond)";
            }

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

        {/* Group layer - render behind nodes */}
        {result.groups?.map((group) => (
          <div
            key={group.id}
            style={{
              position: "absolute",
              left: group.bounds.x,
              top: group.bounds.y,
              width: group.bounds.w,
              height: group.bounds.h,
              backgroundColor: "#252525",
              border: "1px solid #404040",
              borderRadius: 6,
              boxSizing: "border-box",
            }}
          >
            <div
              style={{
                position: "absolute",
                top: -10,
                left: 8,
                backgroundColor: "#252525",
                padding: "0 6px",
                fontSize: 11,
                fontFamily: "Fira Code VF",
                color: "#888",
              }}
            >
              {group.id}
            </div>
          </div>
        ))}

        {/* Node layer */}
        {result.nodes?.map((node) => (
          <div
            key={node.id}
            style={{
              position: "absolute",
              left: node.bounds.x,
              top: node.bounds.y,
              width: node.bounds.w,
              height: node.bounds.h,
              backgroundColor: "#2d2d2d",
              border: "1px solid #555",
              borderRadius: 4,
              padding: 8,
              boxSizing: "border-box",
              fontFamily: "Fira Code VF",
              fontSize: 12,
              color: "#e0e0e0",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                fontWeight: "bold",
                marginBottom: 4,
                borderBottom: "1px solid #444",
                paddingBottom: 4,
                color: "#9CDCFE",
              }}
            >
              {node.label ?? node.id}
            </div>
            {node.body_lines.map((line, i) => (
              <div key={i} style={{ fontSize: 11, color: "#aaa" }}>
                {line}
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
