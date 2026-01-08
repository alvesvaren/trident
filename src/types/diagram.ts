/**
 * Shared type definitions for the diagram system
 */

export interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type TextElement =
  | { type: "Stereotype"; data: { text: string; y: number; font_size: number } }
  | { type: "Title"; data: { text: string; y: number; font_size: number; italic: boolean } }
  | { type: "Separator"; data: { x1: number; y1: number; x2: number; y2: number } }
  | { type: "BodyText"; data: { text: string; y: number; font_size: number } };

export interface NodeRenderingConfig {
  padding: number;
  line_height: number;
  separator_spacing: number;
  char_width: number;
}

export interface DiagramNode {
  id: string;
  /** Node kind: "class" or "node" */
  kind: string;
  /** Modifiers: "abstract", "interface", "enum", "rectangle", "circle", "diamond", etc. */
  modifiers: string[];
  label: string | null;
  text_elements: TextElement[];
  rendering_config: NodeRenderingConfig;
  bounds: Bounds;
  has_pos: boolean;
  parent_offset: { x: number; y: number };
  /** Whether this node was explicitly declared (false for implicit nodes from relations) */
  explicit: boolean;
}

export interface DiagramEdge {
  from: string;
  to: string;
  arrow: string;
  label: string | null;
}

export interface DiagramGroup {
  id: string;
  bounds: Bounds;
}

/** Error information for Monaco editor markers */
export interface ErrorInfo {
  message: string;
  line: number; // 1-based line number
  column: number; // 1-based column number
  end_line: number; // 1-based end line
  end_column: number; // 1-based end column
}

export interface DiagramOutput {
  groups?: DiagramGroup[];
  nodes?: DiagramNode[];
  edges?: DiagramEdge[];
  /** List of implicit node IDs (for editor info diagnostics) */
  implicit_nodes?: string[];
  error?: ErrorInfo;
}

/** Drag state for tracking node/group dragging */
export interface DragState {
  type: "node" | "group" | "resize";
  id: string;
  groupIndex?: number;
  startX: number;
  startY: number;
  startMouseX: number;
  startMouseY: number;
  parentOffsetX: number;
  parentOffsetY: number;
  currentX: number;
  currentY: number;
  // Resize specific
  resizeHandle?: string;
  startW?: number;
  startH?: number;
  initialX?: number;
  initialY?: number;
  newX?: number;
  newY?: number;
  newW?: number;
  newH?: number;
}
