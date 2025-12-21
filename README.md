# Trident

## What is this?

Trident is a domain specific language for creating diagrams. It is similar to mermaid but focused on UML diagrams for now, and is a bit simpler overall.

This allows for the rendering and updating of diagrams in just a few milliseconds, and struggles way less with very complex diagrams.
It also allows for manual positioning of nodes, which isn't possible for classDiagrams in mermaid.

Demo: https://trd.svaren.dev/

The project is currently just a POC that I've used a lot of vibe-coding to build, so the code is pretty horrible. I plan to refactor it and make it more maintainable in the future, if I find the time.

## Features

The project consists of a rust wasm library for parsing the language and layouting the diagram, and a web app in react that uses this library. The web app features a monaco editor with some simple language integrations like syntax highlighting and some basic autocompletion. Also allows for renaming symbols and has error messages.

## Developing

Run `pnpm dev` after installing dependencies. This will start a deveserver that will automatically rebuild and update when the rust code or react code changes!

You might need to run `cargo install cargo-watch wasm-pack` for the dev server to work.

## Example of a diagram

```trd
class ThisIsAClass {
    @pos: (50, 50)
}

group Model {
    class E "Example" {
        + someField: string
        - someOtherField: number

        + someMethod(): Color
    }

    enum Color {
        Red
        Green
        Blue
    }

    class A
    class B

    E .. A
    E --> B

    E ..> Color
}
```
