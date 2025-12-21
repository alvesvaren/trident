import { useRef, useCallback } from "react";
import { Save, FolderOpen, Trash2, Unlock } from "lucide-react";
import * as trident_core from "trident-core";

interface ToolbarProps {
    code: string;
    onCodeChange: (code: string) => void;
}

export function Toolbar({ code, onCodeChange }: ToolbarProps) {
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleSave = useCallback(() => {
        const blob = new Blob([code], { type: "text/plain" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "untitled.trd";
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }, [code]);

    const handleLoad = useCallback(() => {
        fileInputRef.current?.click();
    }, []);

    const handleFileChange = useCallback(
        (e: React.ChangeEvent<HTMLInputElement>) => {
            const file = e.target.files?.[0];
            if (file) {
                const reader = new FileReader();
                reader.onload = (event) => {
                    const content = event.target?.result as string;
                    onCodeChange(content);
                };
                reader.readAsText(file);
            }
            e.target.value = "";
        },
        [onCodeChange]
    );

    const handleClear = useCallback(() => {
        onCodeChange("");
    }, [onCodeChange]);

    const handleRemoveAllLocks = useCallback(() => {
        const newCode = trident_core.remove_all_pos(code);
        onCodeChange(newCode);
    }, [code, onCodeChange]);

    return (
        <div
            className="h-12 shrink-0 flex items-center px-3 gap-2"
            style={{
                backgroundColor: "var(--control-bg)",
                borderTop: "1px solid var(--control-border)",
            }}
        >
            <input
                type="file"
                ref={fileInputRef}
                className="hidden"
                accept=".trd,.txt"
                onChange={handleFileChange}
            />
            <ToolbarButton icon={<Save size={14} />} label="Save" onClick={handleSave} />
            <ToolbarButton icon={<FolderOpen size={14} />} label="Load" onClick={handleLoad} />
            <ToolbarButton icon={<Trash2 size={14} />} label="Clear" onClick={handleClear} />
            <ToolbarButton
                icon={<Unlock size={14} />}
                label="Remove All Locks"
                onClick={handleRemoveAllLocks}
            />
        </div>
    );
}

interface ToolbarButtonProps {
    icon: React.ReactNode;
    label: string;
    onClick: () => void;
}

function ToolbarButton({ icon, label, onClick }: ToolbarButtonProps) {
    return (
        <button
            className="flex items-center gap-1.5 px-3 py-1.5 rounded font-mono text-xs transition-colors"
            style={{
                backgroundColor: "var(--control-bg)",
                border: "1px solid var(--control-border)",
                color: "var(--control-text)",
            }}
            onClick={onClick}
        >
            {icon}
            {label}
        </button>
    );
}
