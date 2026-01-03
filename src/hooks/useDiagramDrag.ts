import { useState, useCallback, useRef, useEffect } from "react";
import * as trident_core from "trident-core";
import type { DiagramNode, DiagramGroup, DragState, DiagramOutput } from "../types/diagram";
import type { CodeEditorRef } from "../components/editor/CodeEditor";

const DRAG_THROTTLE_MS = 10;

interface UseDiagramDragOptions {
  code: string;
  onCodeChange: (code: string) => void;
  /** Optional editor ref for silent updates (no undo history during drag) */
  editorRef?: React.RefObject<CodeEditorRef | null>;
}

interface UseDiagramDragResult {
  dragState: DragState | null;
  /** Layout result computed during drag (use this when dragState is not null) */
  dragResult: DiagramOutput | null;
  scaleRef: React.MutableRefObject<number>;
  startNodeDrag: (e: React.MouseEvent, node: DiagramNode) => void;
  startGroupDrag: (e: React.MouseEvent, group: DiagramGroup, index: number) => void;
}

export function useDiagramDrag({ code, onCodeChange, editorRef }: UseDiagramDragOptions): UseDiagramDragResult {
  const [dragState, setDragState] = useState<DragState | null>(null);
  // Layout result computed during drag (to avoid updating React code state)
  const [dragResult, setDragResult] = useState<DiagramOutput | null>(null);
  const scaleRef = useRef<number>(1);
  const codeRef = useRef(code);
  codeRef.current = code;
  // Track the current code during drag (separate from React state)
  const dragCodeRef = useRef<string | null>(null);
  const lastLayoutUpdateRef = useRef<number>(0);
  const lastUpdateRef = useRef<{ x: number; y: number } | null>(null);
  // Track if we've made any updates during this drag (to know if we need undo stop)
  const hasUpdatedRef = useRef(false);
  // Track pending code that we're waiting to be applied (to prevent flicker on release)
  const pendingCodeRef = useRef<string | null>(null);

  // Clear dragResult once the parent code has been updated to match our pending code
  useEffect(() => {
    if (pendingCodeRef.current && code === pendingCodeRef.current) {
      pendingCodeRef.current = null;
      setDragResult(null);
    }
  }, [code]);

  const startNodeDrag = useCallback(
    (e: React.MouseEvent, node: DiagramNode) => {
      e.preventDefault();
      e.stopPropagation();
      hasUpdatedRef.current = false;
      // Use pendingCodeRef if available (handles race condition when starting new drag
      // before React state has updated from previous drag)
      let sourceCode = pendingCodeRef.current ?? codeRef.current;

      // If node is implicit, insert a declaration first
      if (!node.explicit) {
        const localX = node.bounds.x - node.parent_offset.x;
        const localY = node.bounds.y - node.parent_offset.y;
        sourceCode = trident_core.insert_implicit_node(sourceCode, node.id, localX, localY);
      }

      dragCodeRef.current = sourceCode;
      // Push undo stop before starting drag to mark the "before" state
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
      });
    },
    [editorRef]
  );

  const startGroupDrag = useCallback(
    (e: React.MouseEvent, group: DiagramGroup, index: number) => {
      e.preventDefault();
      e.stopPropagation();
      hasUpdatedRef.current = false;
      // Use pendingCodeRef if available (handles race condition when starting new drag
      // before React state has updated from previous drag)
      dragCodeRef.current = pendingCodeRef.current ?? codeRef.current;
      // Push undo stop before starting drag to mark the "before" state
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

  // Use document-level event listeners to prevent dropping when moving fast
  useEffect(() => {
    if (!dragState) return;

    // Helper function to update the code/layout
    const updateLayout = (currentDrag: DragState, isFinal: boolean) => {
      const newLocalX = currentDrag.currentX - currentDrag.parentOffsetX;
      const newLocalY = currentDrag.currentY - currentDrag.parentOffsetY;

      // Skip if position hasn't changed since last update
      if (lastUpdateRef.current && lastUpdateRef.current.x === newLocalX && lastUpdateRef.current.y === newLocalY) {
        return;
      }
      lastUpdateRef.current = { x: newLocalX, y: newLocalY };

      // Use drag code ref for incremental updates during drag
      const sourceCode = dragCodeRef.current ?? codeRef.current;

      let newCode: string;
      if (currentDrag.type === "node") {
        newCode = trident_core.update_class_pos(sourceCode, currentDrag.id, newLocalX, newLocalY);
      } else {
        newCode = trident_core.update_group_pos(sourceCode, currentDrag.id, currentDrag.groupIndex ?? 0, newLocalX, newLocalY);
      }

      if (newCode !== sourceCode) {
        hasUpdatedRef.current = true;
        dragCodeRef.current = newCode;

        if (isFinal) {
          // On release: update React state (this will sync editor properly)
          // Keep dragResult showing the final position until parent code updates
          // (prevents flicker for one frame)
          const jsonResult = trident_core.compile_diagram(newCode);
          setDragResult(JSON.parse(jsonResult));
          pendingCodeRef.current = newCode;
          onCodeChange(newCode);
        } else if (editorRef?.current) {
          // During drag: update Monaco silently and compile layout locally
          editorRef.current.silentSetValue(newCode);
          // Compile layout locally without updating React code state
          const jsonResult = trident_core.compile_diagram(newCode);
          setDragResult(JSON.parse(jsonResult));
        }
      }
    };

    const handleMouseMove = (e: MouseEvent) => {
      const scale = scaleRef.current;
      const deltaX = (e.clientX - dragState.startMouseX) / scale;
      const deltaY = (e.clientY - dragState.startMouseY) / scale;

      const newX = Math.round(dragState.startX + deltaX);
      const newY = Math.round(dragState.startY + deltaY);

      // Update visual position immediately
      setDragState(prev =>
        prev
          ? {
              ...prev,
              currentX: newX,
              currentY: newY,
            }
          : null
      );

      // Throttled layout update
      const now = Date.now();
      if (now - lastLayoutUpdateRef.current >= DRAG_THROTTLE_MS) {
        lastLayoutUpdateRef.current = now;
        setDragState(currentDrag => {
          if (currentDrag) {
            updateLayout(currentDrag, false);
          }
          return currentDrag; // Keep the drag state
        });
      }
    };

    const handleMouseUp = () => {
      setDragState(currentDrag => {
        if (!currentDrag) return null;

        // Final update on mouse up
        updateLayout(currentDrag, true);

        // Push undo stop after drag ends to mark the "after" state
        // This groups all drag updates into a single undo operation
        if (hasUpdatedRef.current && editorRef?.current) {
          editorRef.current.pushUndoStop();
        }

        lastUpdateRef.current = null;
        lastLayoutUpdateRef.current = 0;
        dragCodeRef.current = null;

        return null;
      });
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragState?.startMouseX, dragState?.startMouseY, dragState?.startX, dragState?.startY, onCodeChange, editorRef]);

  return {
    dragState,
    dragResult,
    scaleRef,
    startNodeDrag,
    startGroupDrag,
  };
}
