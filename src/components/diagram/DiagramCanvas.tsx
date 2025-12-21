import { useCallback, useRef, useMemo, useState, useEffect } from "react";
import { TransformWrapper, TransformComponent, useControls } from "react-zoom-pan-pinch";
import { ZoomIn, ZoomOut, RotateCcw, Download, Image, Home, Focus, Maximize2, Minimize2 } from "lucide-react";
import * as trident_core from "trident-core";
import type { DiagramOutput } from "../../types/diagram";
import { useDiagramDrag } from "../../hooks/useDiagramDrag";
import { SVGNode } from "./SVGNode";
import { SVGGroup } from "./SVGGroup";
import { EdgeDefs, SVGEdges } from "./SVGEdges";
import type { CodeEditorRef } from "../editor/CodeEditor";

interface DiagramCanvasProps {
    result: DiagramOutput;
    code: string;
    onCodeChange: (code: string) => void;
    /** Editor ref for silent updates during drag (no undo history) */
    editorRef?: React.RefObject<CodeEditorRef | null>;
}

interface ZoomControlsProps {
    onExportSVG: () => void;
    onExportPNG: () => void;
    containerRef: React.RefObject<HTMLDivElement | null>;
    svgViewport: { x: number; y: number; width: number; height: number };
}

function ZoomControls({ onExportSVG, onExportPNG, containerRef, svgViewport }: ZoomControlsProps) {
    const { zoomIn, zoomOut, resetTransform, centerView, setTransform } = useControls();
    const [isFullscreen, setIsFullscreen] = useState(false);

    // Listen for fullscreen changes
    useEffect(() => {
        const handleFullscreenChange = () => {
            setIsFullscreen(document.fullscreenElement === containerRef.current);
        };
        document.addEventListener("fullscreenchange", handleFullscreenChange);
        return () => document.removeEventListener("fullscreenchange", handleFullscreenChange);
    }, [containerRef]);

    // Fit entire diagram to viewport
    const handleFitToScreen = useCallback(() => {
        if (!containerRef.current) return;
        const container = containerRef.current;
        const containerWidth = container.clientWidth;
        const containerHeight = container.clientHeight;

        // Calculate scale to fit the entire diagram with some padding
        const padding = 40;
        const scaleX = (containerWidth - padding * 2) / svgViewport.width;
        const scaleY = (containerHeight - padding * 2) / svgViewport.height;
        const scale = Math.min(scaleX, scaleY, 1); // Don't zoom in past 100%

        // Center the diagram
        const centerX = (containerWidth - svgViewport.width * scale) / 2 - svgViewport.x * scale;
        const centerY = (containerHeight - svgViewport.height * scale) / 2 - svgViewport.y * scale;

        setTransform(centerX, centerY, scale, 300);
    }, [containerRef, svgViewport, setTransform]);

    // Reset to 100% scale
    const handleResetScale = useCallback(() => {
        centerView(1, 300);
    }, [centerView]);

    // Toggle fullscreen
    const handleFullscreen = useCallback(() => {
        if (!containerRef.current) return;

        if (document.fullscreenElement) {
            document.exitFullscreen();
        } else {
            containerRef.current.requestFullscreen();
        }
    }, [containerRef]);

    return (
        <div className="absolute top-3 right-3 z-20 flex gap-1">
            <button
                onClick={handleFitToScreen}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Fit to Screen"
            >
                <Home size={16} />
            </button>
            <button
                onClick={handleResetScale}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Reset to 100%"
            >
                <Focus size={16} />
            </button>
            <div className="w-px bg-neutral-700 mx-1" />
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
            <div className="w-px bg-neutral-700 mx-1" />
            <button
                onClick={onExportSVG}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Export as SVG"
            >
                <Download size={16} />
            </button>
            <button
                onClick={onExportPNG}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title="Export as PNG"
            >
                <Image size={16} />
            </button>
            <div className="w-px bg-neutral-700 mx-1" />
            <button
                onClick={handleFullscreen}
                className="p-2 bg-neutral-800 border border-neutral-700 rounded text-neutral-300 hover:bg-neutral-700 transition-colors"
                title={isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
            >
                {isFullscreen ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
            </button>
        </div>
    );
}

export function DiagramCanvas({ result, code, onCodeChange, editorRef }: DiagramCanvasProps) {
    const svgRef = useRef<SVGSVGElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const { dragState, dragResult, scaleRef, startNodeDrag, startGroupDrag } =
        useDiagramDrag({ code, onCodeChange, editorRef });

    // Use dragResult during drag (computed locally), otherwise use the prop result
    const displayResult = dragResult ?? result;

    // Calculate SVG viewport with support for negative coordinates
    const svgViewport = useMemo(() => {
        const nodes = displayResult.nodes ?? [];
        const groups = displayResult.groups ?? [];

        if (nodes.length === 0 && groups.length === 0) {
            return { x: 0, y: 0, width: 800, height: 600 };
        }

        let minX = Infinity;
        let minY = Infinity;
        let maxX = -Infinity;
        let maxY = -Infinity;

        nodes.forEach((n) => {
            minX = Math.min(minX, n.bounds.x);
            minY = Math.min(minY, n.bounds.y);
            maxX = Math.max(maxX, n.bounds.x + n.bounds.w);
            maxY = Math.max(maxY, n.bounds.y + n.bounds.h);
        });

        groups.forEach((g) => {
            minX = Math.min(minX, g.bounds.x);
            minY = Math.min(minY, g.bounds.y);
            maxX = Math.max(maxX, g.bounds.x + g.bounds.w);
            maxY = Math.max(maxY, g.bounds.y + g.bounds.h);
        });

        // Add padding
        const padding = 50;
        return {
            x: minX - padding,
            y: minY - padding,
            width: maxX - minX + padding * 2,
            height: maxY - minY + padding * 2,
        };
    }, [displayResult.nodes, displayResult.groups]);

    // Global keyboard shortcuts for undo/redo (works in fullscreen mode)
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // Only handle when canvas container is in fullscreen
            if (document.fullscreenElement !== containerRef.current) return;

            const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
            const ctrlOrCmd = isMac ? e.metaKey : e.ctrlKey;

            if (ctrlOrCmd && e.key === "z" && !e.shiftKey) {
                e.preventDefault();
                editorRef?.current?.undo();
            } else if (ctrlOrCmd && (e.key === "y" || (e.key === "z" && e.shiftKey))) {
                e.preventDefault();
                editorRef?.current?.redo();
            }
        };

        document.addEventListener("keydown", handleKeyDown);
        return () => document.removeEventListener("keydown", handleKeyDown);
    }, [editorRef]);

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

    const exportSVG = useCallback(() => {
        if (!svgRef.current) return;

        // Clone the SVG for export
        const clone = svgRef.current.cloneNode(true) as SVGSVGElement;

        // Add XML declaration and namespace
        clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");

        // Set a background
        const bg = document.createElementNS("http://www.w3.org/2000/svg", "rect");
        bg.setAttribute("x", String(svgViewport.x));
        bg.setAttribute("y", String(svgViewport.y));
        bg.setAttribute("width", String(svgViewport.width));
        bg.setAttribute("height", String(svgViewport.height));
        bg.setAttribute("fill", "#171717");
        clone.insertBefore(bg, clone.firstChild);

        const svgData = new XMLSerializer().serializeToString(clone);
        const blob = new Blob([svgData], { type: "image/svg+xml" });
        const url = URL.createObjectURL(blob);

        const link = document.createElement("a");
        link.href = url;
        link.download = "diagram.svg";
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);
    }, [svgViewport]);

    const exportPNG = useCallback(async () => {
        if (!svgRef.current) return;

        // Clone the SVG for export
        const clone = svgRef.current.cloneNode(true) as SVGSVGElement;
        clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");

        // Add background
        const bg = document.createElementNS("http://www.w3.org/2000/svg", "rect");
        bg.setAttribute("x", String(svgViewport.x));
        bg.setAttribute("y", String(svgViewport.y));
        bg.setAttribute("width", String(svgViewport.width));
        bg.setAttribute("height", String(svgViewport.height));
        bg.setAttribute("fill", "#171717");
        clone.insertBefore(bg, clone.firstChild);

        const svgData = new XMLSerializer().serializeToString(clone);
        const svgBase64 = btoa(unescape(encodeURIComponent(svgData)));
        const imgSrc = `data:image/svg+xml;base64,${svgBase64}`;

        const img = new window.Image();
        img.onload = () => {
            const canvas = document.createElement("canvas");
            const scale = 2; // 2x resolution for crisp export
            canvas.width = svgViewport.width * scale;
            canvas.height = svgViewport.height * scale;
            const ctx = canvas.getContext("2d");
            if (!ctx) return;

            ctx.scale(scale, scale);
            ctx.translate(-svgViewport.x, -svgViewport.y);
            ctx.drawImage(img, svgViewport.x, svgViewport.y, svgViewport.width, svgViewport.height);

            canvas.toBlob((blob) => {
                if (!blob) return;
                const url = URL.createObjectURL(blob);
                const link = document.createElement("a");
                link.href = url;
                link.download = "diagram.png";
                document.body.appendChild(link);
                link.click();
                document.body.removeChild(link);
                URL.revokeObjectURL(url);
            }, "image/png");
        };
        img.src = imgSrc;
    }, [svgViewport]);

    return (
        <div ref={containerRef} className="relative h-full bg-neutral-900 overflow-hidden">
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
                <ZoomControls onExportSVG={exportSVG} onExportPNG={exportPNG} containerRef={containerRef} svgViewport={svgViewport} />
                <TransformComponent
                    wrapperStyle={{ width: "100%", height: "100%" }}
                    contentStyle={{ width: "100%", height: "100%" }}
                >
                    <svg
                        ref={svgRef}
                        style={{
                            position: "absolute",
                            top: svgViewport.y,
                            left: svgViewport.x,
                            overflow: "visible",
                        }}
                        width={svgViewport.width}
                        height={svgViewport.height}
                        viewBox={`${svgViewport.x} ${svgViewport.y} ${svgViewport.width} ${svgViewport.height}`}
                        className={dragState ? "cursor-grabbing" : ""}
                    >
                        <defs>
                            <EdgeDefs />
                        </defs>

                        {displayResult.error && (
                            <text x={50} y={50} fill="#ef4444" fontSize={14}>
                                {displayResult.error.message}
                            </text>
                        )}

                        {/* Groups (background layer) */}
                        {displayResult.groups?.map((group, index) => {
                            const isDragging = dragState?.type === "group" && dragState.id === group.id;
                            const x = isDragging ? dragState!.currentX : group.bounds.x;
                            const y = isDragging ? dragState!.currentY : group.bounds.y;

                            return (
                                <SVGGroup
                                    key={group.id}
                                    group={group}
                                    x={x}
                                    y={y}
                                    onMouseDown={(e) => startGroupDrag(e as unknown as React.MouseEvent, group, index)}
                                />
                            );
                        })}

                        {/* Edges (middle layer) */}
                        {displayResult.nodes && displayResult.edges && (
                            <SVGEdges
                                edges={displayResult.edges}
                                nodes={displayResult.nodes}
                                dragState={dragState}
                            />
                        )}

                        {/* Nodes (top layer) */}
                        {displayResult.nodes?.map((node) => {
                            const isDragging = dragState?.type === "node" && dragState.id === node.id;
                            const x = isDragging ? dragState!.currentX : node.bounds.x;
                            const y = isDragging ? dragState!.currentY : node.bounds.y;

                            return (
                                <SVGNode
                                    key={node.id}
                                    node={node}
                                    x={x}
                                    y={y}
                                    onMouseDown={(e) => startNodeDrag(e as unknown as React.MouseEvent, node)}
                                    onUnlock={(e) => handleUnlock(node.id, e as unknown as React.MouseEvent)}
                                />
                            );
                        })}
                    </svg>
                </TransformComponent>
            </TransformWrapper>
        </div>
    );
}
