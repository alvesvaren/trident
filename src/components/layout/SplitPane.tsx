import { useState, useCallback, useRef, useEffect, type ReactNode } from "react";
import "./SplitPane.css";

interface SplitPaneProps {
    left: ReactNode;
    right: ReactNode;
    minLeftWidth?: number;
    minRightWidth?: number;
    defaultLeftWidth?: number;
}

export function SplitPane({
    left,
    right,
    minLeftWidth = 300,
    minRightWidth = 300,
    defaultLeftWidth = window.innerWidth / 2,
}: SplitPaneProps) {
    const [leftWidth, setLeftWidth] = useState(defaultLeftWidth);
    const [isDragging, setIsDragging] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    const handleMouseDown = useCallback((e: React.MouseEvent) => {
        e.preventDefault();
        setIsDragging(true);
    }, []);

    useEffect(() => {
        if (!isDragging) return;

        const handleMouseMove = (e: MouseEvent) => {
            if (!containerRef.current) return;

            const containerRect = containerRef.current.getBoundingClientRect();
            const containerWidth = containerRect.width;
            const newLeftWidth = e.clientX - containerRect.left;

            // Enforce min widths
            const maxLeftWidth = containerWidth - minRightWidth;
            const clampedWidth = Math.max(minLeftWidth, Math.min(maxLeftWidth, newLeftWidth));

            setLeftWidth(clampedWidth);
        };

        const handleMouseUp = () => {
            setIsDragging(false);
        };

        document.addEventListener("mousemove", handleMouseMove);
        document.addEventListener("mouseup", handleMouseUp);

        return () => {
            document.removeEventListener("mousemove", handleMouseMove);
            document.removeEventListener("mouseup", handleMouseUp);
        };
    }, [isDragging, minLeftWidth, minRightWidth]);

    return (
        <div ref={containerRef} className="split-pane">
            <div className="split-pane-left" style={{ width: leftWidth }}>
                {left}
            </div>
            <div
                className={`split-pane-handle ${isDragging ? "split-pane-handle-active" : ""}`}
                onMouseDown={handleMouseDown}
            />
            <div className="split-pane-right">
                {right}
            </div>
        </div>
    );
}
