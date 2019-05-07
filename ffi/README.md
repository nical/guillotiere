# Guillotière ffi

<p align="center">
  <a href="https://crates.io/crates/guillotiere_ffi">
      <img src="http://meritbadge.herokuapp.com/guillotiere_ffi" alt="crates.io">
  </a>
  <a href="https://travis-ci.org/nical/guillotiere">
      <img src="https://img.shields.io/travis/nical/guillotiere/master.svg" alt="Travis Build Status">
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

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
