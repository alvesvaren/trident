import { useEffect, useState } from "react";
import { compile_diagram } from "trident-core";

import Editor from "@monaco-editor/react";
import { registerSddLanguage } from "./syntax";

interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface DiagramNode {
  id: string;
  label: string | null;
  body_lines: string[];
  bounds: Bounds;
}

interface DiagramOutput {
  nodes?: DiagramNode[];
  error?: string;
}

function App() {
  const [code, setCode] = useState("");
  const [result, setResult] = useState<DiagramOutput>({});

  useEffect(() => {
    const start = performance.now();
    const jsonResult = compile_diagram(code);
    setResult(JSON.parse(jsonResult));
    const end = performance.now();
    console.log(`Time taken to parse: ${end - start} milliseconds`);
  }, [code]);

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      <Editor
        beforeMount={registerSddLanguage}
        language="trident"
        theme="trident-dark"
        height="100vh"
        width="50vw"
        value={code}
        options={{
          minimap: { enabled: false },
          fontLigatures: false,
          fontFamily: "Fira Code VF",
        }}
        onChange={(value) => setCode(value ?? "")}
      />
      <div
        id="diagram"
        style={{
          flex: 1,
          position: "relative",
          overflow: "auto",
          backgroundColor: "#1e1e1e",
        }}
      >
        {result.error && (
          <div style={{ color: "#f44", padding: 16 }}>{result.error}</div>
        )}
        {result.nodes?.map((node) => (
          <div
            key={node.id}
            style={{
              position: "absolute",
              left: node.bounds.x,
              top: node.bounds.y,
              width: node.bounds.w,
              height: node.bounds.h,
              backgroundColor: "#2d2d2d",
              border: "1px solid #555",
              borderRadius: 4,
              padding: 8,
              boxSizing: "border-box",
              fontFamily: "Fira Code VF",
              fontSize: 12,
              color: "#e0e0e0",
              overflow: "hidden",
            }}
          >
            <div style={{
              fontWeight: "bold",
              marginBottom: 4,
              borderBottom: "1px solid #444",
              paddingBottom: 4,
              color: "#9CDCFE",
            }}>
              {node.label ?? node.id}
            </div>
            {node.body_lines.map((line, i) => (
              <div key={i} style={{ fontSize: 11, color: "#aaa" }}>{line}</div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
