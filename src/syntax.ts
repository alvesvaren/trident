// sddMonaco.ts
// Monaco language support for SDD/Trident:
// - keywords: classDiagram, group, class, interface, enum, struct, record, trait, object
// - modifiers: abstract, static, sealed, final, public, private, protected
// - comments: %% line comment
// - strings: "..." (no escapes)
// - relations: support arrow tokens even without spaces (A-->B, A<|--B:label)
// - directive: @pos: (x, y)
// - braces: { }
// - identifiers: [A-Za-z_][A-Za-z0-9_]*
//
// Usage with @monaco-editor/react is shown below.

import type * as monaco from "monaco-editor";

export const TRIDENT_ID = "trident";

export function registerSddLanguage(monacoApi: typeof monaco) {
  // 1) Register language
  monacoApi.languages.register({ id: TRIDENT_ID });

  // 2) Language configuration (brackets, comments, auto-closing, etc.)
  monacoApi.languages.setLanguageConfiguration(TRIDENT_ID, {
    comments: {
      lineComment: "%%",
    },
    brackets: [
      ["{", "}"],
      ["(", ")"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    folding: {
      offSide: false,
      markers: {
        start: new RegExp("^\\s*(group|class|interface|enum|struct|record|trait|object)\\b.*\\{\\s*$"),
        end: new RegExp("^\\s*}\\s*$"),
      },
    },
  });

  // 3) Monarch tokenizer
  monacoApi.languages.setMonarchTokensProvider(TRIDENT_ID, {
    defaultToken: "",
    tokenPostfix: ".sdd",

    // Node kind keywords
    nodeKinds: ["class", "interface", "enum", "struct", "record", "trait", "object"],

    // Modifier keywords
    modifiers: ["abstract", "static", "sealed", "final", "public", "private", "protected"],

    // Other keywords
    keywords: ["classDiagram", "group"],

    // Arrow tokens (longest first)
    arrows: [
      "<|--",
      "--|>",
      "..>",
      "<..",
      "---",
      "-->",
      "<--",
      "o--",
      "*--",
      "..",
    ],

    tokenizer: {
      root: [
        // line comment
        [/%%.*$/, "comment"],

        // directive (currently only @pos:)
        [/[@]pos:/, "annotation"],

        // braces / parens
        [/[{}]/, "@brackets"],
        [/[()]/, "@brackets"],

        // numbers (for @pos coords)
        [/-?\d+/, "number"],

        // strings (no escapes per v0.0.1)
        [/"/, { token: "string.quote", bracket: "@open", next: "@string" }],

        // arrow operators (including when embedded in A-->B)
        // We highlight them anywhere in the line; Monaco will match mid-token.
        [/<\|--|--\|>|\.\.>|<\.\.|----|-->|<--|o--|\*--|\.\./, "operator"],

        // label delimiter in relations (A-->B:label)
        [/:/, "delimiter"],

        // Node kinds (highlighted specially)
        [/\b(class|interface|enum|struct|record|trait|object)\b/, "keyword.type"],

        // Modifiers (highlighted specially)
        [/\b(abstract|static|sealed|final|public|private|protected)\b/, "keyword.modifier"],

        // Other keywords
        [/\b(classDiagram|group)\b/, "keyword"],

        // identifiers
        [/[A-Za-z_][A-Za-z0-9_]*/, "identifier"],

        // commas
        [/\,/, "delimiter"],

        // whitespace
        [/\s+/, "white"],
      ],

      string: [
        [/[^"]+/, "string"],
        [/"/, { token: "string.quote", bracket: "@close", next: "@pop" }],
      ],
    },
  });

  monacoApi.editor.defineTheme("trident-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      // Keywords
      { token: "keyword", foreground: "C586C0" },
      { token: "keyword.type", foreground: "4EC9B0", fontStyle: "bold" },  // Node kinds in teal
      { token: "keyword.modifier", foreground: "569CD6" },  // Modifiers in blue

      // Annotations/directives
      { token: "annotation", foreground: "DCDCAA" },

      // Other tokens
      { token: "comment", foreground: "6A9955" },
      { token: "string", foreground: "CE9178" },
      { token: "number", foreground: "B5CEA8" },
      { token: "operator", foreground: "D4D4D4" },
      { token: "delimiter", foreground: "D4D4D4" },
      { token: "identifier", foreground: "9CDCFE" },
    ],
    colors: {
      // Keep default editor colors
    },
  });
}
