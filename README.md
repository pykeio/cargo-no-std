## `cargo no-std`: Check which dependencies are & aren't no_std compatible
`cargo no-std` spits out a nice overview of whether your dependencies are `no_std` compatible.

```
carson@desktop:~/example$ cargo no-std
✅ example
❓ bincode
  ❯ Source code contains an explicit `use std::` statement.
   → /home/carson/.cargo/registry/src/github.com-1ecc6299db9ec823/bincode-2.0.0-rc.1/src/features/impl_std.rs
✅ num-traits
❌ rayon
  ❯ Did not find a #![no_std] attribute or a simple conditional attribute like #![cfg_attr(not(feature = "std"), no_std)] in the crate source. Crate most likely doesn't support no_std without changes.
❓ tracing
  ★ Crate supports no_std if "std" feature is deactivated.
    ❯ Caused by feature flag "std" in crate "tracing:0.1.35"
      ❯ Caused by feature flag "default" in crate "tracing:0.1.35"
        ❯ Caused by implicitly enabled default feature from "ml2:1.0.0-dev.20220511"
```

## Installation
```bash
$ cargo install cargo-no-std
```

## License

Licensed under either of

  * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
  * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
