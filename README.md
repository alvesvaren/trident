# trident

## Developing

Run `pnpm dev` after installing dependencies. This will start a deveserver that will automatically rebuild and update when the rust code or react code changes!


## Trident language:

```trd
group Model {
    class Example {
        + someField: string
        - someOtherField: number

        + someMethod()
    }
}

```