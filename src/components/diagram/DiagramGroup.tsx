import type { DiagramGroup as DiagramGroupType } from "../../types/diagram";

interface DiagramGroupProps {
    group: DiagramGroupType;
    x: number;
    y: number;
    onMouseDown: (e: React.MouseEvent) => void;
}

export function DiagramGroup({ group, x, y, onMouseDown }: DiagramGroupProps) {
    return (
        <div
            className="absolute bg-neutral-800 border border-neutral-700 rounded-md box-border cursor-grab active:cursor-grabbing"
            style={{
                left: x,
                top: y,
                width: group.bounds.w,
                height: group.bounds.h,
            }}
            onMouseDown={onMouseDown}
        >
            <div className="absolute -top-2.5 left-2 bg-neutral-800 px-1.5 text-[11px] font-mono text-neutral-500 pointer-events-none">
                {group.id}
            </div>
        </div>
    );
}
