import { useEffect, useState, useRef } from "react";
import * as trident_core from "trident-core";
import type { DiagramOutput } from "./types/diagram";
import { SplitPane } from "./components/layout/SplitPane";
import { CodeEditor, type CodeEditorRef } from "./components/editor/CodeEditor";
import { Toolbar } from "./components/editor/Toolbar";
import { DiagramCanvas } from "./components/diagram/DiagramCanvas";

const STORAGE_KEY = "trident-editor-code";

function App() {
  const [code, setCode] = useState(() => localStorage.getItem(STORAGE_KEY) ?? "");
  const [result, setResult] = useState<DiagramOutput>({});
  const editorRef = useRef<CodeEditorRef | null>(null);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, code);
  }, [code]);

  useEffect(() => {
    const start = performance.now();
    const jsonResult = trident_core.compile_diagram(code);
    setResult(JSON.parse(jsonResult));
    const end = performance.now();
    console.log(`Time taken to parse: ${end - start} milliseconds`);
  }, [code]);

  return (
    <SplitPane
      minLeftWidth={300}
      minRightWidth={300}
      left={
        <>
          <div style={{ flex: 1, overflow: "hidden" }}>
            <CodeEditor ref={editorRef} value={code} onChange={setCode} error={result.error} implicitNodes={result.implicit_nodes} />
          </div>
          <Toolbar code={code} onCodeChange={setCode} />
        </>
      }
      right={<DiagramCanvas result={result} code={code} onCodeChange={setCode} editorRef={editorRef} />}
    />
  );
}

export default App;
