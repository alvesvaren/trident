# Trident

## What is this?

Trident is a domain specific language for creating diagrams. It is similar to mermaid but focused on UML diagrams for now, and is a bit simpler overall.

This allows for the rendering and updating of diagrams in just a few milliseconds, and struggles way less with very complex diagrams.
It also allows for manual positioning of nodes, which isn't possible for classDiagrams in mermaid.

Demo: https://trd.svaren.dev/

The project is currently just a POC that I've used a lot of vibe-coding to build, so the code is pretty horrible. I plan to refactor it and make it more maintainable in the future, if I find the time.

## Features

### Language Features

#### Node Types
- **Class types**: `class`, `interface`, `enum`, `struct`, `record`, `trait`, `object`
- **Shape nodes**: `node`, `rectangle`, `circle`, `diamond`
- Nodes can have optional display labels: `class MyClass "Display Name"`
- Nodes can have bodies with fields and methods:
  ```trd
  class Example {
      + publicField: string
      - privateField: number
      + publicMethod(): ReturnType
  }
  ```

#### Modifiers
- **Class modifiers**: `abstract`, `static`, `sealed`, `final`
- **Visibility modifiers** (for class members): `public` (`+`), `private` (`-`), `protected` (`#`)

#### Groups
- **Named groups**: `group Model { ... }` - creates a visual container with a label
- **Anonymous groups**: `group { ... }` - creates a layout scope without visual container
- Groups can be nested and support manual positioning

#### Directives
- **`@pos: (x, y)`** - Manually position a node or group (relative to parent)
- **`@layout: hierarchical`** or **`@layout: grid`** - Set the layout algorithm for the diagram
- **`@width: value`** - Set custom width for a node
- **`@height: value`** - Set custom height for a node

#### Relations (Arrows)
Trident supports a comprehensive set of UML relation types:

- **`-->`** - Association (solid arrow)
- **`--)`** - Short association (rounded arrow)
- **`--|>`** - Inheritance/extends (hollow triangle)
- **`..|>`** - Implements/realizes (dashed with hollow triangle)
- **`..>`** - Dependency (dashed arrow)
- **`*--`** - Composition (filled diamond at source)
- **`o--`** - Aggregation (hollow diamond at source)
- **`---`** - Simple line (non-directional)
- **`..`** - Dotted line (non-directional)

All directional arrows support left variants (e.g., `<--`, `<|--`, `<|..`).

Relations can include labels: `A --> B : label`

Relations can be written with or without spaces: `A-->B` or `A --> B`

#### Comments
- Line comments: `%% This is a comment`

#### Layout Algorithms
- **Hierarchical** (default) - Graph-driven layout that places connected nodes closer together, respecting hierarchy
- **Grid** - Simple left-to-right, top-to-bottom grid layout

### Editor Features

The web app includes a Monaco editor with:
- **Syntax highlighting** - Color-coded keywords, types, modifiers, and operators
- **Autocompletion** - Smart suggestions for keywords, node types, arrows, and defined symbols
- **Symbol renaming** - Press F2 to rename symbols across the entire diagram
- **Error messages** - Real-time parsing errors with line numbers
- **Code folding** - Fold/unfold groups and node bodies
- **Dark/Light themes** - Built-in theme support

### Technical Architecture

- **Rust WASM core** - High-performance parsing and layout engine compiled to WebAssembly
- **React frontend** - Modern web UI with zoom, pan, and interactive diagram editing
- **Fast rendering** - Diagrams update in milliseconds, even with complex layouts

## Developing

Run `pnpm dev` after installing dependencies. This will start a deveserver that will automatically rebuild and update when the rust code or react code changes!

You might need to run `cargo install cargo-watch wasm-pack` for the dev server to work.


## Simple digram
```trd
A --> B

circle C
C --* D

C --- B
```

## Example of a diagram more complex diagram

```trd
@layout: hierarchical

%% Manually positioned class
class ThisIsAClass {
    @pos: (50, 50)
}

%% Named group with multiple elements
group Model {
    %% Class with display label and body
    class E "Example" {
        + someField: string
        - someOtherField: number
        # protectedField: boolean
        + someMethod(): Color
        - privateMethod(): void
    }

    %% Enum with values
    enum Color {
        Red
        Green
        Blue
    }

    %% Simple class declarations
    class A
    class B

    %% Abstract class
    abstract class BaseClass

    %% Interface
    interface IExample

    %% Relations with different arrow types
    E .. A
    E --> B
    E --|> BaseClass
    E ..|> IExample
    E ..> Color
    E *-- Component
    E o-- Container
}
node Component {
    @pos: (-125, 384)
}
node Container {
    @pos: (-125, 494)
}
```

### More Examples

**Grid Layout:**
```trd
@layout: grid

class A
class B
class C

A --> B
B --> C
```

**Shape Nodes:**
```trd
rectangle Start
circle Process
diamond Decision
node End

Start --> Process
Process --> Decision
Decision --> End
```

**Nested Groups:**
```trd
group Outer {
    group Inner {
        class Nested
    }
    class OuterClass
    
    OuterClass --> Nested
}
```
