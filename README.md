# Decdnnf-rs

Rust tools for Decision-DNNF formulas.

## Compiling/installing decdnnf-rs

Decdnnf-rs requires a recent version of the Rust toolchain (>= 1.72.1).
See [rust-lang.org](https://www.rust-lang.org/tools/install) for more information on how to install Rust.

To build from source, run `cargo build --release` to compile the binary. It will be set in the `target/release` directory.

## How to use

The decdnnf-rs tool expects a subcommand.
To get the list, just invoke the command with the help flag.

```
decdnnf_rs -h
```

In the same way, you can obtain the list of expected and optional arguments by adding the help flag after the subcommand.

```
decdnnf_rs model-counting -h
```

The `model-counting` and the `translation` commands share most of their options, like input file and logging level.
Another one of interest is `--n-vars`. Since the output format of d4 does not provide the number of variables of the problems, it cannot be deduced if it is more important than the highest variable index in use. Setting `--n-vars` allows to override the number of variables returned by the parser, which is set to the highest variable index.

## License

Decdnnf-rs is developed at CRIL (Univ. Artois & CNRS).
It is made available under the terms of the GNU GPLv3 license.
