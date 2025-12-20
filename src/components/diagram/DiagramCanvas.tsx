import { useCallback } from "react";
import * as trident_core from "trident-core";
import type { DiagramOutput } from "../../types/diagram";
import { useDiagramDrag } from "../../hooks/useDiagramDrag";
import { DiagramNode } from "./DiagramNode";
import { DiagramGroup } from "./DiagramGroup";
import { EdgeOverlay } from "./EdgeOverlay";
import "./DiagramCanvas.css";

interface DiagramCanvasProps {
    result: DiagramOutput;
    code: string;
    onCodeChange: (code: string) => void;
}

export function DiagramCanvas({ result, code, onCodeChange }: DiagramCanvasProps) {
    const { dragState, startNodeDrag, startGroupDrag, handleMouseMove, handleMouseUp } =
        useDiagramDrag({ code, onCodeChange });

    const handleUnlock = useCallback(
        (nodeId: string, e: React.MouseEvent) => {
            e.preventDefault();
            e.stopPropagation();
            const newCode = trident_core.remove_class_pos(code, nodeId);
            if (newCode !== code) {
                onCodeChange(newCode);
            }
        },
        [code, onCodeChange]
    );

    return (
        <div
            className={`diagram-canvas ${dragState ? "diagram-canvas-dragging" : ""}`}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
        >
            {result.error && <div className="diagram-error">{result.error}</div>}

            {result.nodes && result.edges && (
                <EdgeOverlay
                    edges={result.edges}
                    nodes={result.nodes}
                    dragState={dragState}
                />
            )}

            {result.groups?.map((group, index) => {
                const isDragging = dragState?.type === "group" && dragState.id === group.id;
                const x = isDragging ? dragState!.currentX : group.bounds.x;
                const y = isDragging ? dragState!.currentY : group.bounds.y;

                return (
                    <DiagramGroup
                        key={group.id}
                        group={group}
                        x={x}
                        y={y}
                        onMouseDown={(e) => startGroupDrag(e, group, index)}
                    />
                );
            })}

            {result.nodes?.map((node) => {
                const isDragging = dragState?.type === "node" && dragState.id === node.id;
                const x = isDragging ? dragState!.currentX : node.bounds.x;
                const y = isDragging ? dragState!.currentY : node.bounds.y;

                return (
                    <DiagramNode
                        key={node.id}
                        node={node}
                        x={x}
                        y={y}
                        onMouseDown={(e) => startNodeDrag(e, node)}
                        onUnlock={(e) => handleUnlock(node.id, e)}
                    />
                );
            })}
        </div>
    );
}
