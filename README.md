# warren

A compact, index-based container designed for fast insertion and removal of elements with reuse of freed slots, and efficient iteration over active entries.

`Warren` stores values in contiguous regions and reuses emptied positions
in constant time. This makes it a good fit for workloads with frequent insert/remove cycles and a need to keep iteration over live elements straightforward.

## Guards

The storage policy is controlled by types with the `WarrenGuard` trait. The default
`FlagGuard` tracks occupancy with a bitmask, while `GenerationGuard` adds a
generation counter so stale indices cannot accidentally mutate a newly
inserted value in the same index.

This container is similar to the plf::hive and boost::container::hub containers, though written in Rust.

