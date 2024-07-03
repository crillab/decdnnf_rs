# Decdnnf-rs

Rust tools for Decision-DNNF formulas, including translation, model counting and model enumeration.

## Compiling/installing decdnnf-rs from sources

Decdnnf-rs requires a recent version of the Rust toolchain (>= 1.72.1).
See [rust-lang.org](https://www.rust-lang.org/tools/install) for more information on how to install Rust.

To build from source, run `cargo build --release` to compile the binary. It will be set in the `target/release` directory.

## How to use

The decdnnf-rs tool expects a subcommand.
To get the list, just invoke the command with the help flag.

```bash
decdnnf_rs -h
```

In the same way, you can obtain the list of expected and optional arguments by adding the help flag after the subcommand.

```bash
decdnnf_rs model-counting -h
```

Some options are common to most commands, like the ones dedicated to input file and logging level.
Another one of interest is `--n-vars`.
Since the output format of d4 (which is the default input format of decdnnf_rs) does not provide the number of variables of the problems, this number cannot be deduced if it is more important than the highest variable index in use.
Setting `--n-vars` allows to override the number of variables returned by the parser, which is set to the highest variable index.

## Translate a d4 Decision-DNNF into a c2d Decision-DNNF

Use the `translation` command:

```bash
decdnnf_rs translation -i instance.nnf
```

## Count the models of a Decision-DNNF

Use the `model-counting` command:

```bash
decdnnf_rs model-counting -i instance.nnf
```

## Enumerate the models of a Decision-DNNF

Use the `model-enumeration` command:

```bash
decdnnf_rs model-enumeration -i instance.nnf
```
This commands admits multiple options allowing to set the number of variables (in case it is higher than the highest index in the input formula), use a compact output or use an enumeration algorithm based on a decision tree.
Run `decdnnf_rs model-enumeration -h` for more information.

## License

Decdnnf-rs is developed at CRIL (Univ. Artois & CNRS).
It is made available under the terms of the GNU GPLv3 license.
