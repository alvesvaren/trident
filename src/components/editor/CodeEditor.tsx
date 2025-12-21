import Editor, { type Monaco } from "@monaco-editor/react";
import type * as monaco from "monaco-editor";
import { useImperativeHandle, forwardRef, useRef, useCallback } from "react";
import { registerSddLanguage } from "../../syntax";

export interface CodeEditorRef {
    /** Update the editor content without creating an undo stop (for drag operations) */
    silentSetValue: (value: string) => void;
    /** Push an undo stop (call after drag ends to mark the undo point) */
    pushUndoStop: () => void;
}

interface CodeEditorProps {
    value: string;
    onChange: (value: string) => void;
}

export const CodeEditor = forwardRef<CodeEditorRef, CodeEditorProps>(
    function CodeEditor({ value, onChange }, ref) {
        const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
        // Flag to suppress onChange callback during silent updates
        const suppressOnChangeRef = useRef(false);

        useImperativeHandle(ref, () => ({
            silentSetValue: (newValue: string) => {
                const editor = editorRef.current;
                if (!editor) return;

                const model = editor.getModel();
                if (!model) return;

                // Get the full range of the document
                const fullRange = model.getFullModelRange();

                // Suppress onChange during this edit
                suppressOnChangeRef.current = true;

                // Execute edit without pushing undo stop
                // This groups all drag updates into a single undo action
                editor.executeEdits("drag-update", [{
                    range: fullRange,
                    text: newValue,
                    forceMoveMarkers: true,
                }]);

                // Re-enable onChange after a microtask to ensure the event has fired
                queueMicrotask(() => {
                    suppressOnChangeRef.current = false;
                });
            },
            pushUndoStop: () => {
                const editor = editorRef.current;
                if (editor) {
                    editor.pushUndoStop();
                }
            },
        }), []);

        const handleEditorDidMount = (
            editor: monaco.editor.IStandaloneCodeEditor,
            _monaco: Monaco
        ) => {
            editorRef.current = editor;
        };

        const handleChange = useCallback((newValue: string | undefined) => {
            // Skip if we're in a silent update
            if (suppressOnChangeRef.current) return;
            onChange(newValue ?? "");
        }, [onChange]);

        return (
            <Editor
                beforeMount={registerSddLanguage}
                onMount={handleEditorDidMount}
                language="trident"
                theme="trident-dark"
                height="100%"
                value={value}
                options={{
                    minimap: { enabled: false },
                    fontLigatures: false,
                    fontFamily: "Fira Code VF",
                }}
                onChange={handleChange}
            />
        );
    }
);
