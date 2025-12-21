import { useState, useCallback, useRef, useEffect } from "react";
import * as trident_core from "trident-core";
import type { DiagramNode, DiagramGroup, DragState } from "../types/diagram";

const DRAG_THROTTLE_MS = 10;

interface UseDiagramDragOptions {
    code: string;
    onCodeChange: (code: string) => void;
}

interface UseDiagramDragResult {
    dragState: DragState | null;
    scaleRef: React.MutableRefObject<number>;
    startNodeDrag: (e: React.MouseEvent, node: DiagramNode) => void;
    startGroupDrag: (e: React.MouseEvent, group: DiagramGroup, index: number) => void;
}

export function useDiagramDrag({
    code,
    onCodeChange,
}: UseDiagramDragOptions): UseDiagramDragResult {
    const [dragState, setDragState] = useState<DragState | null>(null);
    const scaleRef = useRef<number>(1);
    const codeRef = useRef(code);
    codeRef.current = code;
    const lastLayoutUpdateRef = useRef<number>(0);
    const lastUpdateRef = useRef<{ x: number; y: number } | null>(null);

    const startNodeDrag = useCallback((e: React.MouseEvent, node: DiagramNode) => {
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
    }, []);

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

    // Use document-level event listeners to prevent dropping when moving fast
    useEffect(() => {
        if (!dragState) return;

        // Helper function to update the code/layout
        const updateLayout = (currentDrag: DragState) => {
            const newLocalX = currentDrag.currentX - currentDrag.parentOffsetX;
            const newLocalY = currentDrag.currentY - currentDrag.parentOffsetY;

            // Skip if position hasn't changed since last update
            if (
                lastUpdateRef.current &&
                lastUpdateRef.current.x === newLocalX &&
                lastUpdateRef.current.y === newLocalY
            ) {
                return;
            }
            lastUpdateRef.current = { x: newLocalX, y: newLocalY };

            let newCode: string;
            if (currentDrag.type === "node") {
                newCode = trident_core.update_class_pos(
                    codeRef.current,
                    currentDrag.id,
                    newLocalX,
                    newLocalY
                );
            } else {
                newCode = trident_core.update_group_pos(
                    codeRef.current,
                    currentDrag.id,
                    currentDrag.groupIndex ?? 0,
                    newLocalX,
                    newLocalY
                );
            }

            if (newCode !== codeRef.current) {
                onCodeChange(newCode);
            }
        };

        const handleMouseMove = (e: MouseEvent) => {
            const scale = scaleRef.current;
            const deltaX = (e.clientX - dragState.startMouseX) / scale;
            const deltaY = (e.clientY - dragState.startMouseY) / scale;

            const newX = Math.round(dragState.startX + deltaX);
            const newY = Math.round(dragState.startY + deltaY);

            // Update visual position immediately
            setDragState((prev) =>
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
                setDragState((currentDrag) => {
                    if (currentDrag) {
                        updateLayout(currentDrag);
                    }
                    return currentDrag; // Keep the drag state
                });
            }
        };

        const handleMouseUp = () => {
            setDragState((currentDrag) => {
                if (!currentDrag) return null;

                // Final update on mouse up
                updateLayout(currentDrag);
                lastUpdateRef.current = null;
                lastLayoutUpdateRef.current = 0;

                return null;
            });
        };

        document.addEventListener("mousemove", handleMouseMove);
        document.addEventListener("mouseup", handleMouseUp);

        return () => {
            document.removeEventListener("mousemove", handleMouseMove);
            document.removeEventListener("mouseup", handleMouseUp);
        };
    }, [dragState?.startMouseX, dragState?.startMouseY, dragState?.startX, dragState?.startY, onCodeChange]);

    return {
        dragState,
        scaleRef,
        startNodeDrag,
        startGroupDrag,
    };
}
