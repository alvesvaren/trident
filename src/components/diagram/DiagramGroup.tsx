import type { DiagramGroup as DiagramGroupType } from "../../types/diagram";
import "./DiagramGroup.css";

interface DiagramGroupProps {
    group: DiagramGroupType;
    x: number;
    y: number;
    onMouseDown: (e: React.MouseEvent) => void;
}

export function DiagramGroup({ group, x, y, onMouseDown }: DiagramGroupProps) {
    return (
        <div
            className="diagram-group"
            style={{
                left: x,
                top: y,
                width: group.bounds.w,
                height: group.bounds.h,
            }}
            onMouseDown={onMouseDown}
        >
            <div className="diagram-group-label">{group.id}</div>
        </div>
    );
}
