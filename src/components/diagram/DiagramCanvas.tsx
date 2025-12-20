import { useCallback } from "react";
import { TransformWrapper, TransformComponent, useControls } from "react-zoom-pan-pinch";
import { ZoomIn, ZoomOut, RotateCcw } from "lucide-react";
import * as trident_core from "trident-core";
import type { DiagramOutput } from "../../types/diagram";
import { useDiagramDrag } from "../../hooks/useDiagramDrag";
import { DiagramNode } from "./DiagramNode";
import { DiagramGroup } from "./DiagramGroup";
import { EdgeOverlay } from "./EdgeOverlay";

interface DiagramCanvasProps {
    result: DiagramOutput;
    code: string;
    onCodeChange: (code: string) => void;
}

function ZoomControls() {
    const { zoomIn, zoomOut, resetTransform } = useControls();

    return (
        <div className="absolute top-3 right-3 z-20 flex gap-1">
            <button
                onClick={() => zoomIn()}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Zoom In"
            >
                <ZoomIn size={16} />
            </button>
            <button
                onClick={() => zoomOut()}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Zoom Out"
            >
                <ZoomOut size={16} />
            </button>
            <button
                onClick={() => resetTransform()}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Reset View"
            >
                <RotateCcw size={16} />
            </button>
        </div>
    );
}

export function DiagramCanvas({ result, code, onCodeChange }: DiagramCanvasProps) {
    const { dragState, scaleRef, startNodeDrag, startGroupDrag } =
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
        <div className="relative h-full bg-neutral-900 overflow-hidden">
            <TransformWrapper
                initialScale={1}
                minScale={0.25}
                maxScale={4}
                limitToBounds={false}
                panning={{ disabled: dragState !== null }}
                wheel={{ step: 0.1 }}
                onTransformed={(_, state) => {
                    scaleRef.current = state.scale;
                }}
            >
                <ZoomControls />
                <TransformComponent
                    wrapperStyle={{ width: "100%", height: "100%" }}
                    contentStyle={{ width: "100%", height: "100%" }}
                >
                    <div
                        className={`min-w-full min-h-full relative ${dragState ? "cursor-grabbing" : ""}`}
                    >
                        {result.error && (
                            <div className="text-red-500 p-4">{result.error}</div>
                        )}

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
                </TransformComponent>
            </TransformWrapper>
        </div>
    );
}
