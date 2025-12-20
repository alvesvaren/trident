import { Lock } from "lucide-react";
import type { DiagramNode as DiagramNodeType } from "../../types/diagram";
import "./DiagramNode.css";

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
            className="diagram-node"
            style={{
                left: x,
                top: y,
                width: node.bounds.w,
                height: node.bounds.h,
            }}
            onMouseDown={onMouseDown}
        >
            <div className="diagram-node-header">
                <span>{node.label ?? node.id}</span>
                {node.has_pos && (
                    <Lock
                        size={12}
                        className="diagram-node-lock"
                        onClick={onUnlock}
                    />
                )}
            </div>
            {node.body_lines.map((line, i) => (
                <div key={i} className="diagram-node-body-line">
                    {line}
                </div>
            ))}
        </div>
    );
}
