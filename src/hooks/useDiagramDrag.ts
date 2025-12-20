import { useState, useCallback, useRef } from "react";
import * as trident_core from "trident-core";
import type { DiagramNode, DiagramGroup, DragState } from "../types/diagram";

interface UseDiagramDragOptions {
    code: string;
    onCodeChange: (code: string) => void;
}

interface UseDiagramDragResult {
    dragState: DragState | null;
    scaleRef: React.MutableRefObject<number>;
    startNodeDrag: (e: React.MouseEvent, node: DiagramNode) => void;
    startGroupDrag: (e: React.MouseEvent, group: DiagramGroup, index: number) => void;
    handleMouseMove: (e: React.MouseEvent) => void;
    handleMouseUp: () => void;
}

export function useDiagramDrag({
    code,
    onCodeChange,
}: UseDiagramDragOptions): UseDiagramDragResult {
    const [dragState, setDragState] = useState<DragState | null>(null);
    const scaleRef = useRef<number>(1);

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

    const handleMouseMove = useCallback(
        (e: React.MouseEvent) => {
            if (!dragState) return;

            // Divide by scale to get correct delta in diagram coordinates
            const scale = scaleRef.current;
            const deltaX = (e.clientX - dragState.startMouseX) / scale;
            const deltaY = (e.clientY - dragState.startMouseY) / scale;

            setDragState((prev) =>
                prev
                    ? {
                        ...prev,
                        currentX: Math.round(prev.startX + deltaX),
                        currentY: Math.round(prev.startY + deltaY),
                    }
                    : null
            );
        },
        [dragState]
    );

    const handleMouseUp = useCallback(() => {
        if (!dragState) return;

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
            onCodeChange(newCode);
        }
        setDragState(null);
    }, [dragState, code, onCodeChange]);

    return {
        dragState,
        scaleRef,
        startNodeDrag,
        startGroupDrag,
        handleMouseMove,
        handleMouseUp,
    };
}
