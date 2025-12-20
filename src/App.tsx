import { useEffect, useMemo, useState, useCallback, useRef } from "react";
import * as trident_core from "trident-core";
import { Lock, Save, FolderOpen, Trash2, Unlock } from "lucide-react";

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
  has_pos: boolean;
  parent_offset: { x: number; y: number };
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

/** Drag state for tracking node/group dragging */
interface DragState {
  type: "node" | "group";
  id: string;
  groupIndex?: number; // For anonymous groups
  startX: number; // Original position of element (world coords)
  startY: number;
  startMouseX: number; // Mouse position at drag start
  startMouseY: number;
  parentOffsetX: number; // Parent group's world position
  parentOffsetY: number;
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
    const jsonResult = trident_core.compile_diagram(code);
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

  // Drag state
  const [dragState, setDragState] = useState<DragState & { currentX: number; currentY: number } | null>(null);
  const diagramRef = useRef<HTMLDivElement>(null);

  // Start dragging a node
  const startNodeDrag = useCallback(
    (e: React.MouseEvent, node: DiagramNode) => {
      e.preventDefault();
      e.stopPropagation();
      setDragState({
        type: "node",
        id: node.id,
        startX: node.bounds.x,
        startY: node.bounds.y,
        startMouseX: e.clientX,
        startMouseY: e.clientY,
        parentOffsetX: node.parent_offset.x,
        parentOffsetY: node.parent_offset.y,
        currentX: node.bounds.x,
        currentY: node.bounds.y,
      });
    },
    []
  );

  // Start dragging a group
  const startGroupDrag = useCallback(
    (e: React.MouseEvent, group: DiagramGroup, index: number) => {
      e.preventDefault();
      e.stopPropagation();
      setDragState({
        type: "group",
        id: group.id,
        groupIndex: index,
        startX: group.bounds.x,
        startY: group.bounds.y,
        startMouseX: e.clientX,
        startMouseY: e.clientY,
        parentOffsetX: 0,
        parentOffsetY: 0,
        currentX: group.bounds.x,
        currentY: group.bounds.y,
      });
    },
    []
  );

  // Handle mouse move during drag
  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!dragState) return;

      const deltaX = e.clientX - dragState.startMouseX;
      const deltaY = e.clientY - dragState.startMouseY;

      setDragState(prev => prev ? ({
        ...prev,
        currentX: Math.round(prev.startX + deltaX),
        currentY: Math.round(prev.startY + deltaY)
      }) : null);
    },
    [dragState]
  );

  // Handle mouse up to end drag
  const handleMouseUp = useCallback(() => {
    if (!dragState) return;

    // Commit final position
    const newLocalX = dragState.currentX - dragState.parentOffsetX;
    const newLocalY = dragState.currentY - dragState.parentOffsetY;

    let newCode: string;
    if (dragState.type === "node") {
      newCode = trident_core.update_class_pos(code, dragState.id, newLocalX, newLocalY);
    } else {
      newCode = trident_core.update_group_pos(
        code,
        dragState.id,
        dragState.groupIndex ?? 0,
        newLocalX,
        newLocalY
      );
    }

    if (newCode !== code) {
      setCode(newCode);
    }
    setDragState(null);
  }, [dragState, code]);

  // Unlock a node (remove its @pos)
  const handleUnlock = useCallback(
    (nodeId: string, e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const newCode = trident_core.remove_class_pos(code, nodeId);
      if (newCode !== code) {
        setCode(newCode);
      }
    },
    [code]
  );

  // File input ref for loading files
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Save file
  const handleSave = useCallback(() => {
    const blob = new Blob([code], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "untitled.trd";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [code]);

  // Load file
  const handleLoad = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (event) => {
        const content = event.target?.result as string;
        setCode(content);
      };
      reader.readAsText(file);
    }
    // Reset the input so the same file can be loaded again
    e.target.value = "";
  }, []);

  // Clear editor
  const handleClear = useCallback(() => {
    setCode("");
  }, []);

  // Remove all locks
  const handleRemoveAllLocks = useCallback(() => {
    const newCode = trident_core.remove_all_pos(code);
    setCode(newCode);
  }, [code]);

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      <div style={{ display: "flex", flexDirection: "column", width: "50vw" }}>
        <Editor
          beforeMount={registerSddLanguage}
          language="trident"
          theme="trident-dark"
          height="calc(100vh - 48px)"
          value={code}
          options={{
            minimap: { enabled: false },
            fontLigatures: false,
            fontFamily: "Fira Code VF",
          }}
          onChange={(value) => setCode(value ?? "")}
        />
        {/* Toolbar */}
        <div
          style={{
            height: 48,
            backgroundColor: "#252525",
            borderTop: "1px solid #404040",
            display: "flex",
            alignItems: "center",
            padding: "0 12px",
            gap: 8,
          }}
        >
          <input
            type="file"
            ref={fileInputRef}
            style={{ display: "none" }}
            accept=".trd,.txt"
            onChange={handleFileChange}
          />
          <button
            onClick={handleSave}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "6px 12px",
              backgroundColor: "#3d3d3d",
              border: "1px solid #555",
              borderRadius: 4,
              color: "#e0e0e0",
              cursor: "pointer",
              fontFamily: "Fira Code VF",
              fontSize: 12,
            }}
          >
            <Save size={14} /> Save
          </button>
          <button
            onClick={handleLoad}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "6px 12px",
              backgroundColor: "#3d3d3d",
              border: "1px solid #555",
              borderRadius: 4,
              color: "#e0e0e0",
              cursor: "pointer",
              fontFamily: "Fira Code VF",
              fontSize: 12,
            }}
          >
            <FolderOpen size={14} /> Load
          </button>
          <button
            onClick={handleClear}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "6px 12px",
              backgroundColor: "#3d3d3d",
              border: "1px solid #555",
              borderRadius: 4,
              color: "#e0e0e0",
              cursor: "pointer",
              fontFamily: "Fira Code VF",
              fontSize: 12,
            }}
          >
            <Trash2 size={14} /> Clear
          </button>
          <button
            onClick={handleRemoveAllLocks}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "6px 12px",
              backgroundColor: "#3d3d3d",
              border: "1px solid #555",
              borderRadius: 4,
              color: "#e0e0e0",
              cursor: "pointer",
              fontFamily: "Fira Code VF",
              fontSize: 12,
            }}
          >
            <Unlock size={14} /> Remove All Locks
          </button>
        </div>
      </div>
      <div
        id="diagram"
        ref={diagramRef}
        style={{
          flex: 1,
          position: "relative",
          overflow: "auto",
          backgroundColor: "#1e1e1e",
          cursor: dragState ? "grabbing" : "default",
        }}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
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
            const fromNode = result.nodes?.find(n => n.id === edge.from);
            const toNode = result.nodes?.find(n => n.id === edge.to);

            let fromBounds = nodeMap.get(edge.from);
            let toBounds = nodeMap.get(edge.to);

            if (dragState?.type === "node" && fromNode && dragState.id === fromNode.id) {
              fromBounds = { ...fromNode.bounds, x: dragState.currentX, y: dragState.currentY };
            }
            if (dragState?.type === "node" && toNode && dragState.id === toNode.id) {
              toBounds = { ...toNode.bounds, x: dragState.currentX, y: dragState.currentY };
            }

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
        {result.groups?.map((group, index) => {
          const isDragging = dragState?.type === "group" && dragState.id === group.id;
          const x = isDragging ? dragState!.currentX : group.bounds.x;
          const y = isDragging ? dragState!.currentY : group.bounds.y;

          return (
            <div
              key={group.id}
              style={{
                position: "absolute",
                left: x,
                top: y,
                width: group.bounds.w,
                height: group.bounds.h,
                backgroundColor: "#252525",
                border: "1px solid #404040",
                borderRadius: 6,
                boxSizing: "border-box",
                cursor: "grab",
              }}
              onMouseDown={(e) => startGroupDrag(e, group, index)}
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
                  pointerEvents: "none", // Allow drag from label area
                }}
              >
                {group.id}
              </div>
            </div>
          );
        })}

        {/* Node layer */}
        {result.nodes?.map((node) => {
          const isDragging = dragState?.type === "node" && dragState.id === node.id;
          const x = isDragging ? dragState!.currentX : node.bounds.x;
          const y = isDragging ? dragState!.currentY : node.bounds.y;

          return (
            <div
              key={node.id}
              style={{
                position: "absolute",
                left: x,
                top: y,
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
                cursor: "grab",
                userSelect: "none",
              }}
              onMouseDown={(e) => startNodeDrag(e, node)}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  fontWeight: "bold",
                  marginBottom: 4,
                  borderBottom: "1px solid #444",
                  paddingBottom: 4,
                  color: "#9CDCFE",
                }}
              >
                <span>{node.label ?? node.id}</span>
                {node.has_pos && (
                  <Lock
                    size={12}
                    style={{ cursor: "pointer", color: "#888" }}
                    onClick={(e) => handleUnlock(node.id, e)}
                  />
                )}
              </div>
              {node.body_lines.map((line, i) => (
                <div key={i} style={{ fontSize: 11, color: "#aaa" }}>
                  {line}
                </div>
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default App;
