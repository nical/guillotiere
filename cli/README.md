## Guillotière command-line interface

A command line interface for Guillotière's atlas allcator.

The state of the atlas is serialized into a [ron](https://crates.io/crates/ron)
file and can be dumped as an SVG for visualization.

## Commands

If running guillotiere with cargo, replace `guillotiere` with `cargo run --` in all of the examples below.

### init

```bash
# initializes an empty atlas of size 1024x1024
guillotiere init 1024 1024
```

Run `guillotiere init --help` for more options.

### allocate

This command allocates a rectangle of size 100x50 in the atlas.
A name for the rectangle is generated automatically and will be needed for
deallocation.

```bash
guillotiere allocate 100 50
```

To specify the name manually, use the `--name <NAME>` option:

```bash
guillotiere allocate 100 50 --name foo
```

### deallocate

This command Deallocates a rectangle of the specified name from the atlas.

```bash
guillotiere deallocate foo
```

### rearrange

This command reallocates all rectangles to reduce fragmentation.
The size can be optionally modiffed with `-w <width>` and `-h <height>`

```bash
guillotiere rearrange
```


### svg

This command generates an SVG file of the atlas. Green rectangles are free and
blue rectangles are allocated.

```bash
# Writes into ./atlas.svg by default.
guillotiere svg
```

To specify the path of the SVG file:

```bash
guillotiere svg path/to/file.svg
```

### list

This command dumps a list of the allocated and free rectangles in stdout.

```bash
guillotiere list
```

### More options

- All of the commands allow specifying the atlas file with the `-a`/`--atlas` option. By default, the atlas is read/written into `./atlas.ron`.
- Most commands allow generating an SVG file directly using the `--svg <FILE>` option.

Example:

```bash
guillotiere allocate 100 200 --atlas myatlas.ron --svg atlas.svg
```

See `guillotiere --help` and `guillotiere <command> --help` for the other options available.

