# Guillotière

<p align="center">
  <a href="https://crates.io/crates/guillotiere">
      <img src="http://meritbadge.herokuapp.com/guillotiere" alt="crates.io">
  </a>
  <a href="https://travis-ci.org/nical/guillotiere">
      <img src="https://img.shields.io/travis/nical/guillotiere/master.svg" alt="Travis Build Status">
  </a>
  <a href="https://docs.rs/guillotiere">
      <img src="https://docs.rs/guillotiere/badge.svg" alt="documentation">
  </a>

</p>

A dynamic texture atlas allocator with fast deallocation and rectangle coalescing.

## Motivation

The ability to dynamically batch textures together is important for some graphics rendering scenarios (for example [WebRender](https://github.com/servo/webrender)).
A challenging aspect of dynamic atlas allocation is the need to coalesce free rectangles after deallocation to defragment the available space.
Some atlas allocators perform this task by examining all possible pairs of free rectangles and test if they can be merged, which is prohibitively expensive for real-time applications.

Guillotière solves this problem by internally maintaining a data structure that allows constant time acces to neighbor rectangles and greatly speeds up the coalesing operation.

The details of how this works are explained in the [`AtlasAllocator` documentation](https://docs.rs/guillotiere/*/guillotiere/struct.AtlasAllocator.html).

