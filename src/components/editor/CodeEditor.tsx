import Editor, { type Monaco } from "@monaco-editor/react";
import type * as monaco from "monaco-editor";
import { useImperativeHandle, forwardRef, useRef, useCallback, useEffect } from "react";
import { registerSddLanguage } from "../../syntax";
import type { ErrorInfo } from "../../types/diagram";
import { useTheme } from "../../hooks/useTheme";

export interface CodeEditorRef {
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
  const { resolvedTheme } = useTheme();
  const editorTheme = resolvedTheme === "dark" ? "trident-dark" : "trident-light";

  useImperativeHandle(
    ref,
    () => ({
      pushUndoStop: () => {
        editorRef.current?.pushUndoStop();
      },
      undo: () => {
        editorRef.current?.trigger("keyboard", "undo", null);
      },
      redo: () => {
        editorRef.current?.trigger("keyboard", "redo", null);
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
