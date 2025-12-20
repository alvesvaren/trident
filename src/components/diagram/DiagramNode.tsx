import { Lock } from "lucide-react";
import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

interface DiagramNodeProps {
    node: DiagramNodeType;
    x: number;
    y: number;
    onMouseDown: (e: React.MouseEvent) => void;
    onUnlock: (e: React.MouseEvent) => void;
}

export function DiagramNode({ node, x, y, onMouseDown, onUnlock }: DiagramNodeProps) {
    return (
        <div
            className="absolute bg-neutral-800 border border-neutral-600 rounded p-2 box-border font-mono text-xs text-neutral-200 overflow-hidden cursor-grab active:cursor-grabbing select-none"
            style={{
                left: x,
                top: y,
                width: node.bounds.w,
                height: node.bounds.h,
            }}
            onMouseDown={onMouseDown}
        >
            <div className="flex justify-between items-center font-bold mb-1 border-b border-neutral-700 pb-1 text-blue-300">
                <span>{node.label ?? node.id}</span>
                {node.has_pos && (
                    <Lock
                        size={12}
                        className="cursor-pointer text-neutral-500 hover:text-neutral-300 transition-colors"
                        onClick={onUnlock}
                    />
                )}
            </div>
            {node.body_lines.map((line, i) => (
                <div key={i} className="text-[11px] text-neutral-400">
                    {line}
                </div>
            ))}
        </div>
    );
}
