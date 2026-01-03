import Editor, { type Monaco } from "@monaco-editor/react";
import type * as monaco from "monaco-editor";
import { useImperativeHandle, forwardRef, useRef, useCallback, useEffect } from "react";
import { registerSddLanguage } from "../../syntax";
import type { ErrorInfo } from "../../types/diagram";
import { useTheme } from "../../hooks/useTheme";

export interface CodeEditorRef {
  /** Get the current editor value (always up-to-date, even during silent updates) */
  getValue: () => string;
  /** Update the editor content without creating an undo stop (for drag operations) */
  silentSetValue: (value: string) => void;
  /** Push an undo stop (call after drag ends to mark the undo point) */
  pushUndoStop: () => void;
  /** Trigger undo */
  undo: () => void;
  /** Trigger redo */
  redo: () => void;
}

interface CodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  error?: ErrorInfo;
  /** List of implicit node IDs to show info markers for */
  implicitNodes?: string[];
}

export const CodeEditor = forwardRef<CodeEditorRef, CodeEditorProps>(function CodeEditor({ value, onChange, error, implicitNodes }, ref) {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  // Flag to suppress onChange callback during silent updates
  const suppressOnChangeRef = useRef(false);
  const { resolvedTheme } = useTheme();
  const editorTheme = resolvedTheme === "dark" ? "trident-dark" : "trident-light";

  useImperativeHandle(
    ref,
    () => ({
      getValue: () => {
        const editor = editorRef.current;
        if (!editor) return "";
        const model = editor.getModel();
        if (!model) return "";
        return model.getValue();
      },
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
        editor.executeEdits("drag-update", [
          {
            range: fullRange,
            text: newValue,
            forceMoveMarkers: true,
          },
        ]);

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
      undo: () => {
        const editor = editorRef.current;
        if (editor) {
          editor.trigger("keyboard", "undo", null);
        }
      },
      redo: () => {
        const editor = editorRef.current;
        if (editor) {
          editor.trigger("keyboard", "redo", null);
        }
      },
    }),
    []
  );

  // Update Monaco markers when error or implicitNodes changes
  useEffect(() => {
    const editor = editorRef.current;
    const monacoInstance = monacoRef.current;
    if (!editor || !monacoInstance) return;

    const model = editor.getModel();
    if (!model) return;

    const markers: monaco.editor.IMarkerData[] = [];

    // Add error marker if present
    if (error) {
      markers.push({
        severity: monacoInstance.MarkerSeverity.Error,
        message: error.message,
        startLineNumber: error.line,
        startColumn: error.column,
        endLineNumber: error.end_line,
        endColumn: error.end_column,
      });
    }

    // Add info markers for implicit nodes (find them in the source)
    if (implicitNodes && implicitNodes.length > 0) {
      const content = model.getValue();
      const lines = content.split("\n");
      for (const nodeId of implicitNodes) {
        // Find all occurrences of the node ID in relations
        const regex = new RegExp(`\\b${nodeId}\\b`, "g");
        for (let lineNum = 0; lineNum < lines.length; lineNum++) {
          const line = lines[lineNum];
          let match;
          while ((match = regex.exec(line)) !== null) {
            // Check if this line looks like a relation (contains arrow)
            if (
              line.includes("-->") ||
              line.includes("<--") ||
              line.includes("<|--") ||
              line.includes("--|>") ||
              line.includes("..>") ||
              line.includes("<..") ||
              line.includes("---") ||
              line.includes("o--") ||
              line.includes("*--")
            ) {
              markers.push({
                severity: monacoInstance.MarkerSeverity.Info,
                message: `Implicit node: '${nodeId}' is not explicitly declared`,
                startLineNumber: lineNum + 1,
                startColumn: match.index + 1,
                endLineNumber: lineNum + 1,
                endColumn: match.index + nodeId.length + 1,
              });
            }
          }
        }
      }
    }

    monacoInstance.editor.setModelMarkers(model, "trident", markers);
  }, [error, implicitNodes]);

  const handleEditorDidMount = (editor: monaco.editor.IStandaloneCodeEditor, monaco: Monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;
  };

  const handleChange = useCallback(
    (newValue: string | undefined) => {
      // Skip if we're in a silent update
      if (suppressOnChangeRef.current) return;
      onChange(newValue ?? "");
    },
    [onChange]
  );

  return (
    <Editor
      beforeMount={registerSddLanguage}
      onMount={handleEditorDidMount}
      language='trident'
      theme={editorTheme}
      height='100%'
      value={value}
      options={{
        minimap: { enabled: false },
        fontLigatures: false,
        fontFamily: "Fira Code VF",
      }}
      onChange={handleChange}
    />
  );
});
