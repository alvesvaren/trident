import { useState, useCallback, useRef, useEffect } from "react";
import * as trident_core from "trident-core";
import type { DiagramNode, DiagramGroup, DragState } from "../types/diagram";
import type { CodeEditorRef } from "../components/editor/CodeEditor";

const DRAG_THROTTLE_MS = 16; // ~60fps

interface UseDiagramDragOptions {
  code: string;
  onCodeChange: (code: string) => void;
  editorRef?: React.RefObject<CodeEditorRef | null>;
}

interface UseDiagramDragResult {
  dragState: DragState | null;
  scaleRef: React.MutableRefObject<number>;
  startNodeDrag: (e: React.MouseEvent, node: DiagramNode) => void;
  startGroupDrag: (e: React.MouseEvent, group: DiagramGroup, index: number) => void;
  startNodeResize: (e: React.MouseEvent, node: DiagramNode, handle: string) => void;
}

/**
 * Simplified drag hook - uses React state as single source of truth.
 * No silent updates, no complex ref synchronization.
 */
export function useDiagramDrag({ code, onCodeChange, editorRef }: UseDiagramDragOptions): UseDiagramDragResult {
  const [dragState, setDragState] = useState<DragState | null>(null);
  const scaleRef = useRef<number>(1);

  // Single code ref - updated from props on each render
  const codeRef = useRef(code);
  codeRef.current = code;

  // Throttle tracking
  const lastUpdateRef = useRef(0);

  // === PURE HELPER: Compute new code based on drag state ===
  const computeNewCode = useCallback((sourceCode: string, drag: DragState): string => {
    if (drag.type === "node") {
      const localX = Math.round(drag.currentX - drag.parentOffsetX);
      const localY = Math.round(drag.currentY - drag.parentOffsetY);
      // -1 for width/height means "don't change"
      return trident_core.update_class_geometry(sourceCode, drag.id, localX, localY, -1, -1);
    }

    if (drag.type === "resize") {
      const newX = drag.newX ?? drag.initialX ?? 0;
      const newY = drag.newY ?? drag.initialY ?? 0;
      const newW = Math.round(drag.newW ?? drag.startW ?? 0);
      const newH = Math.round(drag.newH ?? drag.startH ?? 0);
      const localX = Math.round(newX - drag.parentOffsetX);
      const localY = Math.round(newY - drag.parentOffsetY);
      return trident_core.update_class_geometry(sourceCode, drag.id, localX, localY, newW, newH);
    }

    // Group
    const localX = Math.round(drag.currentX - drag.parentOffsetX);
    const localY = Math.round(drag.currentY - drag.parentOffsetY);
    return trident_core.update_group_pos(sourceCode, drag.id, drag.groupIndex ?? 0, localX, localY);
  }, []);

  // === START HANDLERS ===
  const startNodeDrag = useCallback(
    (e: React.MouseEvent, node: DiagramNode) => {
      e.preventDefault();
      e.stopPropagation();

      let sourceCode = codeRef.current;

      // If node is implicit, insert declaration first
      if (!node.explicit) {
        const localX = node.bounds.x - node.parent_offset.x;
        const localY = node.bounds.y - node.parent_offset.y;
        sourceCode = trident_core.insert_implicit_node(sourceCode, node.id, localX, localY);
        codeRef.current = sourceCode;
        onCodeChange(sourceCode);
      }

      editorRef?.current?.pushUndoStop();

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
        startW: node.bounds.w,
        startH: node.bounds.h,
      });
    },
    [editorRef, onCodeChange]
  );

  const startNodeResize = useCallback(
    (e: React.MouseEvent, node: DiagramNode, handle: string) => {
      e.preventDefault();
      e.stopPropagation();

      editorRef?.current?.pushUndoStop();

      setDragState({
        type: "resize",
        id: node.id,
        startX: node.bounds.w,
        startY: node.bounds.h,
        startMouseX: e.clientX,
        startMouseY: e.clientY,
        resizeHandle: handle,
        currentX: node.bounds.x,
        currentY: node.bounds.y,
        parentOffsetX: node.parent_offset.x,
        parentOffsetY: node.parent_offset.y,
        initialX: node.bounds.x,
        initialY: node.bounds.y,
        startW: node.bounds.w,
        startH: node.bounds.h,
        newX: node.bounds.x,
        newY: node.bounds.y,
        newW: node.bounds.w,
        newH: node.bounds.h,
      });
    },
    [editorRef]
  );

  const startGroupDrag = useCallback(
    (e: React.MouseEvent, group: DiagramGroup, index: number) => {
      e.preventDefault();
      e.stopPropagation();

      editorRef?.current?.pushUndoStop();

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
    [editorRef]
  );

  // === MOUSE TRACKING EFFECT ===
  useEffect(() => {
    if (!dragState) return;

    const handleMouseMove = (e: MouseEvent) => {
      const scale = scaleRef.current;
      const deltaX = (e.clientX - dragState.startMouseX) / scale;
      const deltaY = (e.clientY - dragState.startMouseY) / scale;

      // Compute new state based on drag type
      let newState: DragState;

      if (dragState.type === "resize") {
        const initialW = dragState.startW ?? 0;
        const initialH = dragState.startH ?? 0;
        const initialX = dragState.initialX ?? 0;
        const initialY = dragState.initialY ?? 0;
        const handle = dragState.resizeHandle ?? "";

        let newW = initialW;
        let newH = initialH;
        let newX = initialX;
        let newY = initialY;

        // 8-way resize logic
        if (handle.includes("e")) newW = initialW + deltaX;
        if (handle.includes("w")) {
          newW = initialW - deltaX;
          newX = initialX + deltaX;
        }
        if (handle.includes("s")) newH = initialH + deltaY;
        if (handle.includes("n")) {
          newH = initialH - deltaY;
          newY = initialY + deltaY;
        }

        // Clamp minimum size
        if (newW < 40) {
          if (handle.includes("w")) newX = initialX + initialW - 40;
          newW = 40;
        }
        if (newH < 40) {
          if (handle.includes("n")) newY = initialY + initialH - 40;
          newH = 40;
        }

        newState = { ...dragState, newX, newY, newW, newH, currentX: newX, currentY: newY };
      } else {
        // Move node or group
        const newX = Math.round(dragState.startX + deltaX);
        const newY = Math.round(dragState.startY + deltaY);
        newState = { ...dragState, currentX: newX, currentY: newY };
      }

      setDragState(newState);

      // Throttled code update
      const now = Date.now();
      if (now - lastUpdateRef.current >= DRAG_THROTTLE_MS) {
        lastUpdateRef.current = now;
        const newCode = computeNewCode(codeRef.current, newState);
        if (newCode !== codeRef.current) {
          codeRef.current = newCode;
          onCodeChange(newCode);
        }
      }
    };

    const handleMouseUp = () => {
      // Final code update
      const finalCode = computeNewCode(codeRef.current, dragState);
      if (finalCode !== codeRef.current) {
        codeRef.current = finalCode;
        onCodeChange(finalCode);
      }

      editorRef?.current?.pushUndoStop();
      setDragState(null);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragState, computeNewCode, onCodeChange, editorRef]);

  return {
    dragState,
    scaleRef,
    startNodeDrag,
    startGroupDrag,
    startNodeResize,
  };
}
