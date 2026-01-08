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
import * as trident_core from "trident-core";
import { initArrowRegistry, getArrowRegistry, generateArrowLabel, type ArrowEntry } from "./types/arrows";

export const TRIDENT_ID = "trident";

// Initialize arrow registry from Rust
initArrowRegistry(trident_core);

// Keywords for completion
const KEYWORDS = [
  "class",
  "interface",
  "enum",
  "struct",
  "record",
  "trait",
  "object",
  "node",
  "rectangle",
  "circle",
  "diamond",
  "group",
  "abstract",
  "static",
  "sealed",
  "final",
  "public",
  "private",
  "protected",
];

// Snippets for completion
const SNIPPETS = [
  {
    label: "class",
    insertText: "class ${1:ClassName} {\n\t$0\n}",
    detail: "Class with body",
    documentation: "Create a class with a body block",
  },
  {
    label: "class-simple",
    insertText: "class ${1:ClassName}",
    detail: "Simple class",
    documentation: "Create a simple class without body",
  },
  {
    label: "interface",
    insertText: "interface ${1:InterfaceName} {\n\t$0\n}",
    detail: "Interface with body",
    documentation: "Create an interface with a body block",
  },
  {
    label: "group",
    insertText: "group ${1:GroupName} {\n\t$0\n}",
    detail: "Named group",
    documentation: "Create a named group of elements",
  },
  {
    label: "group-anon",
    insertText: "group {\n\t$0\n}",
    detail: "Anonymous group",
    documentation: "Create an anonymous group",
  },
  {
    label: "abstract-class",
    insertText: "abstract class ${1:ClassName} {\n\t$0\n}",
    detail: "Abstract class",
    documentation: "Create an abstract class",
  },
  {
    label: "enum",
    insertText: "enum ${1:EnumName} {\n\t$0\n}",
    detail: "Enum with body",
    documentation: "Create an enum with values",
  },
  {
    label: "relation",
    insertText: "${1:From} --> ${2:To}",
    detail: "Simple relation",
    documentation: "Create a relation between two elements",
  },
  {
    label: "relation-labeled",
    insertText: "${1:From} --> ${2:To} : ${3:label}",
    detail: "Labeled relation",
    documentation: "Create a labeled relation",
  },
  {
    label: "extends",
    insertText: "${1:Child} <|-- ${2:Parent}",
    detail: "Inheritance relation",
    documentation: "Create an inheritance/extends relation",
  },
];

/** Build arrow completions from the registry */
function getArrowCompletions(): { token: string; label: string; detail: string }[] {
  return getArrowRegistry().map((entry: ArrowEntry) => ({
    token: entry.token,
    label: generateArrowLabel(entry.token, entry.name, entry.is_left),
    detail: entry.detail,
  }));
}

/** Build regex for arrows that contain parentheses (must match before other rules) */
function buildParenArrowRegex(): RegExp {
  const tokens = getArrowRegistry()
    .map((e: ArrowEntry) => e.token)
    .filter(t => t.includes('(') || t.includes(')'));
  if (tokens.length === 0) return /(?!)/; // Never matches
  const sorted = tokens.sort((a, b) => b.length - a.length);
  const escaped = sorted.map(t => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  return new RegExp(`(?:${escaped.join('|')})`);
}

/** Build regex for arrow tokenization (excluding paren arrows, which are handled separately) */
function buildArrowRegex(): RegExp {
  const tokens = getArrowRegistry()
    .map((e: ArrowEntry) => e.token)
    .filter(t => !t.includes('(') && !t.includes(')'));
  if (tokens.length === 0) return /(?!)/; // Never matches
  const sorted = tokens.sort((a, b) => b.length - a.length);
  const escaped = sorted.map(t => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  return new RegExp(escaped.join('|'));
}

export function registerSddLanguage(monacoApi: typeof monaco) {
  // Get arrow data from registry
  const arrowTokens = getArrowRegistry().map((e: ArrowEntry) => e.token);
  const arrowRegex = buildArrowRegex();
  const parenArrowRegex = buildParenArrowRegex();
  const arrowCompletions = getArrowCompletions();

  // 1) Register language
  monacoApi.languages.register({ id: TRIDENT_ID });

  // 2) Language configuration (brackets, comments, auto-closing, etc.)
  monacoApi.languages.setLanguageConfiguration(TRIDENT_ID, {
    comments: {
      lineComment: "%%",
    },
    brackets: [
      ["{", "}"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: '"', close: '"' },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
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
    nodeKinds: ["class", "interface", "enum", "struct", "record", "trait", "object", "node", "rectangle", "circle", "diamond"],

    // Modifier keywords
    modifiers: ["abstract", "static", "sealed", "final", "public", "private", "protected"],

    // Other keywords
    keywords: ["classDiagram", "group"],

    // Arrow tokens (from registry, already sorted by length)
    arrows: arrowTokens,

    tokenizer: {
      root: [
        // line comment
        [/%%.*$/, "comment"],

        // arrow operators with parentheses (must come first to prevent parens from being tokenized separately)
        [parenArrowRegex, "operator"],

        // directive (currently only @pos:)
        [/[@]pos:/, "annotation"],

        // layout directive (@layout: grid, @layout: hierarchical)
        [/[@]layout:/, "annotation"],

        // size directives (@width: and @height:)
        [/[@]width:/, "annotation"],
        [/[@]height:/, "annotation"],

        // braces
        [/[{}]/, "@brackets"],

        // numbers (for @pos coords)
        [/-?\d+/, "number"],

        // strings (no escapes per v0.0.1)
        [/"/, { token: "string.quote", bracket: "@open", next: "@string" }],

        // arrow operators (including when embedded in A-->B)
        [arrowRegex, "operator"],

        // label delimiter in relations (A-->B:label)
        [/:/, "delimiter"],

        // Node kinds (highlighted specially)
        [/\b(class|interface|enum|struct|record|trait|object|node|rectangle|circle|diamond)\b/, "keyword.type"],

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

  // 4) Completion provider (keywords, snippets, relations)
  monacoApi.languages.registerCompletionItemProvider(TRIDENT_ID, {
    // Trigger on arrow chars, space, and all letters for symbol completion
    triggerCharacters: ["-", ".", "<", ">", " ", ..."abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"],
    provideCompletionItems: (model, position) => {
      const word = model.getWordUntilPosition(position);
      const range: monaco.IRange = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      // Get line content up to cursor
      const lineContent = model.getLineContent(position.lineNumber);
      const textBeforeCursor = lineContent.substring(0, position.column - 1);

      const suggestions: monaco.languages.CompletionItem[] = [];

      // Always add matching symbols when typing an identifier
      // This helps with both sides of relations (A --> B) and standalone references
      const source = model.getValue();
      try {
        const symbolsJson = trident_core.get_symbols(source);
        const symbols: string[] = JSON.parse(symbolsJson);
        const lowerWord = word.word.toLowerCase();
        for (const sym of symbols) {
          // Show all symbols if no word typed, or filter by prefix match
          if (word.word === "" || sym.toLowerCase().startsWith(lowerWord)) {
            suggestions.push({
              label: sym,
              kind: monacoApi.languages.CompletionItemKind.Reference,
              insertText: sym,
              range,
              detail: "Defined symbol",
              sortText: "0" + sym, // Sort symbols first
            });
          }
        }
      } catch {
        // Ignore parse errors
      }

      // Check if we're typing an arrow (after identifier and space)
      const arrowTypingMatch = textBeforeCursor.match(/[A-Za-z_][A-Za-z0-9_]*[\s]+([-.<>|*o]*)$/);
      if (arrowTypingMatch) {
        const partialArrow = arrowTypingMatch[1];
        for (const arrow of arrowCompletions) {
          if (arrow.token.startsWith(partialArrow) || partialArrow === "") {
            suggestions.push({
              label: arrow.label,
              kind: monacoApi.languages.CompletionItemKind.Operator,
              insertText: arrow.token + " ",
              range: {
                ...range,
                startColumn: position.column - partialArrow.length,
              },
              detail: arrow.detail,
              sortText: "1" + arrow.token,
            });
          }
        }
      }

      // Add keyword completions
      for (const kw of KEYWORDS) {
        if (kw.startsWith(word.word.toLowerCase()) || word.word === "") {
          suggestions.push({
            label: kw,
            kind: monacoApi.languages.CompletionItemKind.Keyword,
            insertText: kw,
            range,
            sortText: "2" + kw,
          });
        }
      }

      // Add snippet completions
      for (const snippet of SNIPPETS) {
        if (snippet.label.startsWith(word.word.toLowerCase()) || word.word === "") {
          suggestions.push({
            label: snippet.label,
            kind: monacoApi.languages.CompletionItemKind.Snippet,
            insertText: snippet.insertText,
            insertTextRules: monacoApi.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range,
            detail: snippet.detail,
            documentation: snippet.documentation,
            sortText: "3" + snippet.label,
          });
        }
      }

      return { suggestions };
    },
  });

  // 5) Rename provider (F2)
  monacoApi.languages.registerRenameProvider(TRIDENT_ID, {
    provideRenameEdits: (model, position, newName) => {
      const word = model.getWordAtPosition(position);
      if (!word) {
        return { edits: [] };
      }

      const oldName = word.word;
      const source = model.getValue();

      try {
        const newSource = trident_core.rename_symbol(source, oldName, newName);

        // If source unchanged, symbol wasn't found
        if (newSource === source) {
          return { edits: [] };
        }

        // Return a single edit that replaces the entire content
        return {
          edits: [
            {
              resource: model.uri,
              textEdit: {
                range: model.getFullModelRange(),
                text: newSource,
              },
              versionId: model.getVersionId(),
            },
          ],
        };
      } catch {
        return { edits: [] };
      }
    },

    resolveRenameLocation: (model, position) => {
      const word = model.getWordAtPosition(position);
      if (!word) {
        return {
          range: {
            startLineNumber: position.lineNumber,
            startColumn: position.column,
            endLineNumber: position.lineNumber,
            endColumn: position.column,
          },
          text: "",
          rejectReason: "Cannot rename this element",
        };
      }

      // Check if this word is a valid symbol
      const source = model.getValue();
      try {
        const symbolsJson = trident_core.get_symbols(source);
        const symbols: string[] = JSON.parse(symbolsJson);

        if (!symbols.includes(word.word)) {
          return {
            range: {
              startLineNumber: position.lineNumber,
              startColumn: word.startColumn,
              endLineNumber: position.lineNumber,
              endColumn: word.endColumn,
            },
            text: word.word,
            rejectReason: `'${word.word}' is not a defined symbol`,
          };
        }
      } catch {
        // Allow rename attempt even on parse error
      }

      return {
        range: {
          startLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endLineNumber: position.lineNumber,
          endColumn: word.endColumn,
        },
        text: word.word,
      };
    },
  });

  monacoApi.editor.defineTheme("trident-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      // Keywords
      { token: "keyword", foreground: "C586C0" },
      { token: "keyword.type", foreground: "C586C0" }, // Node kinds in teal
      { token: "keyword.modifier", foreground: "C586C0" }, // Modifiers in blue

      // Annotations/directives
      { token: "annotation", foreground: "C586C0" },

      // Other tokens
      { token: "comment", foreground: "777777" },
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

  monacoApi.editor.defineTheme("trident-light", {
    base: "vs",
    inherit: true,
    rules: [
      // Keywords
      { token: "keyword", foreground: "AF00DB" },
      { token: "keyword.type", foreground: "AF00DB" },
      { token: "keyword.modifier", foreground: "AF00DB" },

      // Annotations/directives
      { token: "annotation", foreground: "AF00DB" },

      // Other tokens
      { token: "comment", foreground: "6A9955" },
      { token: "string", foreground: "A31515" },
      { token: "number", foreground: "098658" },
      { token: "operator", foreground: "000000" },
      { token: "delimiter", foreground: "000000" },
      { token: "identifier", foreground: "001080" },
    ],
    colors: {
      // Keep default editor colors
    },
  });
}
