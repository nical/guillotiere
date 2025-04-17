# Guillotière ffi

<p align="center">
  <a href="https://crates.io/crates/guillotiere_ffi">
      <img src="https://img.shields.io/crates/v/guillotiere_ffi.svg" alt="crates.io">
  </a>
  <a href="https://github.com/nical/guillotiere/actions">
      <img src="https://github.com/nical/guillotiere/actions/workflows/main.yml/badge.svg" alt="Build Status">
  </a>
  <a href="https://docs.rs/guillotiere_ffi">
      <img src="https://docs.rs/guillotiere_ffi/badge.svg" alt="documentation">
  </a>

</p>

C compatible Foreign function interface for guillotière.

## Usage

Bindings can be easily generated using `cbindgen`

```bash
cbindgen . -c cbindgen.toml -o guillotiere.h
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/license/mit)

at your option.
