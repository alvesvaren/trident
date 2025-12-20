import { useEffect, useState } from "react";
import { compile_diagram } from "trident-core";

import Editor from "@monaco-editor/react";
import { registerSddLanguage } from "./syntax";

function App() {
  const [code, setCode] = useState("");
  const [result, setResult] = useState({});
  useEffect(() => {
    // console log time taken to parse
    const start = performance.now();
    const result = compile_diagram(code);
    setResult(JSON.parse(result));
    const end = performance.now();
    console.log(`Time taken to parse: ${end - start} milliseconds`);
  }, [code]);

  return (
    <div style={{ display: "flex" }}>
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
      <div id="diagram" style={{ flex: "1", whiteSpace: "pre-wrap", fontFamily: "Fira Code VF" }} dangerouslySetInnerHTML={{ __html: JSON.stringify(result, null, 2) }}></div>
    </div>
  );
}

export default App;
