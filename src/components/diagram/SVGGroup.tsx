import type { DiagramGroup as DiagramGroupType } from "../../types/diagram";

interface SVGGroupProps {
    group: DiagramGroupType;
    x: number;
    y: number;
    onMouseDown: (e: React.MouseEvent<SVGGElement>) => void;
    /** Hide interactive elements for export */
    exportMode?: boolean;
}

export function SVGGroup({ group, x, y, onMouseDown, exportMode = false }: SVGGroupProps) {
    const labelPadding = 6;
    const labelFontSize = 11;

    return (
        <g
            transform={`translate(${x}, ${y})`}
            onMouseDown={onMouseDown}
            style={{ cursor: exportMode ? "default" : "grab" }}
        >
            {/* Group background */}
            <rect
                x={0}
                y={0}
                width={group.bounds.w}
                height={group.bounds.h}
                rx={6}
                ry={6}
                fill="var(--canvas-node-bg)"
                stroke="var(--canvas-border)"
                strokeWidth={1}
            />

            {/* Label background */}
            <rect
                x={8}
                y={-10}
                width={group.id.length * 7 + labelPadding * 2}
                height={20}
                fill="var(--canvas-node-bg)"
            />

            {/* Label text */}
            <text
                x={8 + labelPadding}
                y={4}
                fill="var(--canvas-text-muted)"
                fontSize={labelFontSize}
                fontFamily="ui-monospace, monospace"
            >
                {group.id}
            </text>
        </g>
    );
}
