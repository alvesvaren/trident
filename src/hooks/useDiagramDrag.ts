import { useState, useCallback, useRef, useEffect } from "react";
import * as trident_core from "trident-core";
import type { DiagramNode, DiagramGroup, DragState } from "../types/diagram";
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
  scaleRef: React.MutableRefObject<number>;
  startNodeDrag: (e: React.MouseEvent, node: DiagramNode) => void;
  startGroupDrag: (e: React.MouseEvent, group: DiagramGroup, index: number) => void;
  startNodeResize: (e: React.MouseEvent, node: DiagramNode, handle: string) => void;
}

export function useDiagramDrag({ code, onCodeChange, editorRef }: UseDiagramDragOptions): UseDiagramDragResult {
  const [dragState, setDragState] = useState<DragState | null>(null);
  const scaleRef = useRef<number>(1);

  // Always track the latest code - updated from props AND after silent updates
  // This is the single source of truth for current code state
  const latestCodeRef = useRef(code);
  latestCodeRef.current = code; // Sync from props on every render

  // Track the current code during drag (separate from React state)
  const dragCodeRef = useRef<string | null>(null);
  const lastLayoutUpdateRef = useRef<number>(0);
  // Track if we've made any updates during this drag (to know if we need undo stop)
  const hasUpdatedRef = useRef(false);

  const startNodeDrag = useCallback(
    (e: React.MouseEvent, node: DiagramNode) => {
      e.preventDefault();
      e.stopPropagation();
      hasUpdatedRef.current = false;
      let sourceCode = latestCodeRef.current;

      // If node is implicit, insert a declaration first
      if (!node.explicit) {
        const localX = node.bounds.x - node.parent_offset.x;
        const localY = node.bounds.y - node.parent_offset.y;
        sourceCode = trident_core.insert_implicit_node(sourceCode, node.id, localX, localY);
        // Keep latestCodeRef in sync after this modification
        latestCodeRef.current = sourceCode;
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
        // Helper props for new update logic
        startW: node.bounds.w,
        startH: node.bounds.h,
      });
    },
    [editorRef]
  );

  const startNodeResize = useCallback(
    (e: React.MouseEvent, node: DiagramNode, handle: string) => {
      e.preventDefault();
      e.stopPropagation();
      hasUpdatedRef.current = false;
      dragCodeRef.current = latestCodeRef.current;

      // If node is implicit, we should probably insert it, but for now duplicate the logic or assume it exists
      // (Resizing implicit nodes might be edge case, but safe to assume standard flow)

      editorRef?.current?.pushUndoStop();
      setDragState({
        type: "resize",
        id: node.id,
        startX: node.bounds.w, // kept for compat if needed, but we use startW/startH now
        startY: node.bounds.h,
        startMouseX: e.clientX,
        startMouseY: e.clientY,
        resizeHandle: handle,
        currentX: node.bounds.w,
        currentY: node.bounds.h,
        parentOffsetX: 0,
        parentOffsetY: 0,
        // Explicit initial state
        initialX: node.bounds.x,
        initialY: node.bounds.y,
        startW: node.bounds.w,
        startH: node.bounds.h,
      });
    },
    [editorRef]
  );

  const startGroupDrag = useCallback(
    (e: React.MouseEvent, group: DiagramGroup, index: number) => {
      e.preventDefault();
      e.stopPropagation();
      hasUpdatedRef.current = false;
      dragCodeRef.current = latestCodeRef.current;
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
      // Use drag code ref for incremental updates during drag
      const sourceCode = dragCodeRef.current ?? latestCodeRef.current;
      let newCode: string;

      if (currentDrag.type === "node") {
        // Moving a node
        const newLocalX = currentDrag.currentX - currentDrag.parentOffsetX;
        const newLocalY = currentDrag.currentY - currentDrag.parentOffsetY;
        // We need to pass current width/height to geometry update.
        // Since 'move' doesn't change W/H, we can grab them from where?
        // We don't track W/H in move-state.
        // BUT, we have 'code' which is the source of truth, OR we can look up the node in the diagram result if we had it.
        // HOWEVER, the most robust way (since we are in drag loop) is to pass what we know.
        // Actually, wait. 'update_class_pos' was fine for just moving.
        // But user wants DRY. 'update_class_geometry' overwrites ALL fields.
        // If we don't have W/H, we might overwrite them with 0 if we aren't careful?
        // Ah, the previous implementation of update_node_position didn't touch W/H.
        // update_node_geometry DOES touch them.
        // So for "Node" drag (move only), we need W/H.
        const newLocalXInt = Math.round(newLocalX);
        const newLocalYInt = Math.round(newLocalY);

        // Use the unified function with -1 for W/H to indicate "no change/partial update"
        // This prevents adding @width/@height when just moving a node
        newCode = trident_core.update_class_geometry(sourceCode, currentDrag.id, newLocalXInt, newLocalYInt, -1, -1);
      } else if (currentDrag.type === "resize") {
        // Resizing a node (and potentially moving it if resizing from top/left)
        // currentX/Y in resize state tracks the NEW DIMENSIONS (W, H) or NEW POS?
        // Let's look at handleMouseMove logic below.

        // In Resize state:
        // currentX = new Width
        // currentY = new Height
        // startX = initial Width
        // startY = initial Height

        // But for NW/N/W/SW resize, we ALSO change position.
        // So we need to track X/Y too.
        // Let's update DragState interface to hold currentPosX/Y effectively.
        // But DragState is rigid.
        // Let's calculate everything in handleMouseMove, store in DragState, and read here.

        // Ideally DragState should have: currentX, currentY (Position), currentW, currentH (Size).
        // Existing DragState has currentX/Y.
        // Let's piggyback or assume currentX/Y = W/H for resize?
        // No, we need 4 values.
        // USEDIAGRAMDRAG refactor:
        // Let's assume for Resize:
        // currentX = Width
        // currentY = Height
        // And we need new X, Y.
        // We can store new X/Y in `dragState` if we expand it?
        // Or we can recalculate them here (but that requires mouse deltas which we don't have easily here).
        // Better to expand DragState in the `setDragState` call in mouseMove.

        // Since I can't easily change the type definition in this tool call (it's in types/diagram.ts?),
        // I will assume I can cast or extend it locally if it helps, OR just abuse existing fields?
        // Safest is to calculate everything in mouseMove and maybe repurpose fields?
        // Actually, I can edit types/diagram.ts? No, it's not open.
        // Wait, I can see `DragState` usage. It has `currentX`, `currentY`.
        // I'll stick to: currentX/Y is the position (X/Y).
        // And I'll add `currentW`, `currentH` to the state?
        // If I cannot change type, I'm stuck.
        // Let's check `types/diagram`. If I can't check it, I assume it's limited.
        // Alternative: usage of `any` or strict separation.

        // Let's look at `useDiagramDrag.ts` imports.
        // `import type { ..., DragState } from "../types/diagram";`

        // I will try to update `handleMouseMove` to set the specialized properties,
        // and here I will read them. I'll cast `currentDrag` to `any` to access new props if needed,
        // since I'm implementing the logic here.

        const d = currentDrag;
        const newX = d.newX ?? d.initialX ?? 0;
        const newY = d.newY ?? d.initialY ?? 0;
        const newW = d.newW ?? d.startW ?? 0;
        const newH = d.newH ?? d.startH ?? 0;

        const localX = newX - (d.parentOffsetX ?? 0);
        const localY = newY - (d.parentOffsetY ?? 0);

        newCode = trident_core.update_class_geometry(sourceCode, currentDrag.id, Math.round(localX), Math.round(localY), Math.round(newW), Math.round(newH));
      } else {
        // Groups use update_group_pos (geometry not supported for groups yet/maybe irrelevant)
        const newLocalX = currentDrag.currentX - currentDrag.parentOffsetX;
        const newLocalY = currentDrag.currentY - currentDrag.parentOffsetY;
        newCode = trident_core.update_group_pos(sourceCode, currentDrag.id, currentDrag.groupIndex ?? 0, newLocalX, newLocalY);
      }

      if (newCode !== sourceCode) {
        hasUpdatedRef.current = true;
        dragCodeRef.current = newCode;
        // CRITICAL: Keep latestCodeRef in sync even during silent updates
        // This ensures any subsequent drag starts with the correct code
        latestCodeRef.current = newCode;

        if (isFinal) {
          // Commit the final code change - App.tsx's useMemo will recompile
          onCodeChange(newCode);
        } else if (editorRef?.current) {
          // Silent update during drag - editor shows changes but no React state update
          editorRef.current.silentSetValue(newCode);
        }
      }
    };

    const handleMouseMove = (e: MouseEvent) => {
      const scale = scaleRef.current;
      const deltaX = (e.clientX - dragState.startMouseX) / scale;
      const deltaY = (e.clientY - dragState.startMouseY) / scale;

      if (dragState.type === "resize") {
        // Resize logic
        // startX/Y in resize state = initial Width/Height?
        // Wait, in startNodeResize I set: startX = w, startY = h.
        // But I also need initial Position (X/Y) to calculate top/left resize.
        // In startNodeResize I didn't store initial X/Y. I NEED to.
        // I'll update startNodeResize to store initialX/initialY in `dragState` (using extra props).

        const d = dragState;
        const initialW = d.startW ?? 0;
        const initialH = d.startH ?? 0;
        const initialX = d.initialX ?? 0;
        const initialY = d.initialY ?? 0;
        const handle = d.resizeHandle ?? "";

        let newW = initialW;
        let newH = initialH;
        let newX = initialX;
        let newY = initialY;

        // Apply 8-way logic
        if (handle.includes("e")) {
          newW = initialW + deltaX;
        }
        if (handle.includes("w")) {
          newW = initialW - deltaX;
          newX = initialX + deltaX;
        }
        if (handle.includes("s")) {
          newH = initialH + deltaY;
        }
        if (handle.includes("n")) {
          newH = initialH - deltaY;
          newY = initialY + deltaY;
        }

        // Clamp min size
        if (newW < 40) {
          // If clamping width, we might need to adjust X if dragging West
          if (handle.includes("w")) {
            // The right edge should stay fixed: (newX + newW) == (initialX + deltaX + initialW - deltaX) ?
            // RightEdge = initialX + initialW.
            // If newW is clamped to 40, then newX = RightEdge - 40.
            newX = initialX + initialW - 40;
          }
          newW = 40;
        }
        if (newH < 40) {
          if (handle.includes("n")) {
            // BottomEdge = initialY + initialH
            newY = initialY + initialH - 40;
          }
          newH = 40;
        }

        setDragState(prev =>
          prev
            ? {
                ...prev,
                newX,
                newY,
                newW,
                newH,
                // Update currentX/Y purely for debugging or unused, since we use newX/Y/W/H
                currentX: newX,
                currentY: newY,
              }
            : null
        );
      } else {
        // Move
        const newX = Math.round(dragState.startX + deltaX);
        const newY = Math.round(dragState.startY + deltaY);
        setDragState(prev => (prev ? { ...prev, currentX: newX, currentY: newY } : null));
      }

      // Throttle
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
    scaleRef,
    startNodeDrag,
    startGroupDrag,
    startNodeResize,
  };
}
