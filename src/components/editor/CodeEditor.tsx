import Editor from "@monaco-editor/react";
import { registerSddLanguage } from "../../syntax";

interface CodeEditorProps {
    value: string;
    onChange: (value: string) => void;
}

export function CodeEditor({ value, onChange }: CodeEditorProps) {
    return (
        <Editor
            beforeMount={registerSddLanguage}
            language="trident"
            theme="trident-dark"
            height="100%"
            value={value}
            options={{
                minimap: { enabled: false },
                fontLigatures: false,
                fontFamily: "Fira Code VF",
            }}
            onChange={(newValue) => onChange(newValue ?? "")}
        />
    );
}
