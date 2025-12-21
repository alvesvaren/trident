import { Lock } from "lucide-react";
import type { DiagramNode as DiagramNodeType } from "../../types/diagram";

interface DiagramNodeProps {
    node: DiagramNodeType;
    x: number;
    y: number;
    onMouseDown: (e: React.MouseEvent) => void;
    onUnlock: (e: React.MouseEvent) => void;
}

/** Format node kind for display (e.g., "interface" -> "«interface»") */
function formatKindStereotype(kind: string): string | null {
    // "class" is the default, don't show stereotype
    if (kind === "class") return null;
    return `«${kind}»`;
}

/** Format modifiers for display (e.g., ["abstract", "sealed"] -> "«abstract» «sealed»") */
function formatModifiers(modifiers: string[]): string | null {
    if (modifiers.length === 0) return null;
    return modifiers.map(m => `«${m}»`).join(" ");
}

/** Get border color based on node kind */
function getBorderColor(kind: string): string {
    switch (kind) {
        case "interface":
            return "border-green-500";
        case "enum":
            return "border-purple-500";
        case "struct":
        case "record":
            return "border-orange-500";
        case "trait":
            return "border-cyan-500";
        case "object":
            return "border-yellow-500";
        default:
            return "border-neutral-600"; // class
    }
}

/** Get title color based on node kind */
function getTitleColor(kind: string): string {
    switch (kind) {
        case "interface":
            return "text-green-300";
        case "enum":
            return "text-purple-300";
        case "struct":
        case "record":
            return "text-orange-300";
        case "trait":
            return "text-cyan-300";
        case "object":
            return "text-yellow-300";
        default:
            return "text-blue-300"; // class
    }
}

export function DiagramNode({ node, x, y, onMouseDown, onUnlock }: DiagramNodeProps) {
    const kindStereotype = formatKindStereotype(node.kind);
    const modifierStereotypes = formatModifiers(node.modifiers);
    const borderColor = getBorderColor(node.kind);
    const titleColor = getTitleColor(node.kind);

    // Combine all stereotypes (modifiers first, then kind)
    const allStereotypes = [modifierStereotypes, kindStereotype]
        .filter(Boolean)
        .join(" ");

    return (
        <div
            className={`absolute bg-neutral-800 border ${borderColor} rounded p-2 box-border font-mono text-xs text-neutral-200 overflow-hidden cursor-grab active:cursor-grabbing select-none`}
            style={{
                left: x,
                top: y,
                width: node.bounds.w,
                height: node.bounds.h,
            }}
            onMouseDown={onMouseDown}
        >
            {/* Stereotype line (modifiers + kind) */}
            {allStereotypes && (
                <div className="text-center text-[10px] text-neutral-400 italic mb-0.5">
                    {allStereotypes}
                </div>
            )}

            {/* Node name / label */}
            <div className={`flex justify-between items-center font-bold mb-1 border-b border-neutral-700 pb-1 ${titleColor}`}>
                <span className={node.modifiers.includes("abstract") ? "italic" : ""}>
                    {node.label ?? node.id}
                </span>
                {node.has_pos && (
                    <Lock
                        size={12}
                        className="cursor-pointer text-neutral-500 hover:text-neutral-300 transition-colors"
                        onClick={onUnlock}
                    />
                )}
            </div>

            {/* Body lines */}
            {node.body_lines.map((line, i) => (
                <div key={i} className="text-[11px] text-neutral-400">
                    {line}
                </div>
            ))}
        </div>
    );
}
