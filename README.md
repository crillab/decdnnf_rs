# decdnnf-rs: A Rust Library and CLI Tool for Decision-DNNF Manipulation

`decdnnf-rs` is a comprehensive Rust library for efficiently manipulating Decision-DNNF formulas, accompanied by a companion Command Line Interface (CLI) tool designed for practical interaction and testing.

Decision-DNNF (Decomposable Negation Normal Form) is a knowledge compilation formalism used in artificial intelligence and logic.
It compiles propositional logic formulas into a tractable representation, enabling efficient inference tasks like model checking and counting.
This project provides both a high-performance library for manipulating Decision-DNNF structures and a user-friendly CLI tool for interacting with these formulas.

The `decdnnf-rs` library and its CLI tool are designed to handle formulas produced by the [d4 knowledge compiler](https://www.cril.univ-artois.fr/software/d4/).
Furthermore, the library enables crucial tasks such as model-counting (determining the number of satisfying assignments) and uniform sampling (generating random models according to the formula) directly on the compiled structure.

## Installation

### Installing the CLI tool

The tool is available on [crates.io](https://crates.io/crates/decdnnf_rs). You can install it using Cargo:

```bash
cargo install decdnnf_rs
```

### Using as a dependency in a Rust project

Just type the following command in the root directory of your project to include the latest version of the library:

```bash
cargo add decdnnf_rs
```

### Building from source

If you'd like to build from source:

```bash
git clone https://github.com/crillab/decdnnf_rs.git
cd decdnnf_rs
cargo build --release
```

The binary will be available at `target/release/decdnnf_rs`.

## Available commands

The `decdnnf-rs` application provides multiple commands, each of which is dedicated to a specific task.
To see the full list, invoke the program with the help flag.

```bash
decdnnf_rs -h
```

Similarly, you can obtain a list of the expected and optional arguments by adding the help flag after the subcommand.

```bash
decdnnf_rs model-counting -h
```

### Common options

Some options are common to most commands, such as those dedicated to the input file and the logging level.
Another interesting option is the `--n-vars` option.
The output format of d4, the default input format of `decdnnf_rs`, does not provide the number of variables for the problems.
Therefore, if this number is more important than the highest variable index in use, it cannot be deduced.
Setting `--n-vars` overrides the number of variables returned by the parser, which is set to the highest variable index.

By default, `decdnnf_rs` performs a partial check of the input Decision-DNNF for correctness, focusing on the decomposability property of the disjunction and the determinism of the disjunction node.
However, these checks may take some time.
If you are confident that the formula is correct, you can skip these tests by adding the `--do-not-check` flag.

### Translate a `d4` Decision-DNNF into a `c2d` Decision-DNNF

The [c2d](http://reasoning.cs.ucla.edu/c2d/) knowledge compiler's output format was the standard format used to represent decision-DNNF for a long time.
The `decdnnf-rs` program provides a command to translate a `d4` formula into a `c2d` formula:

```bash
decdnnf_rs translation -i instance.nnf
```

### Check for a model under assumptions

The `compute-model` command checks for the existence of a model.
Assumptions can be provided using the `-a` option:

```bash
decdnnf_rs compute-model -i instance.nnf -a '1 2'
```

### Count the models of a Decision-DNNF

Use the `model-counting` command:

```bash
decdnnf_rs model-counting -i instance.nnf
```

### Enumerate the models of a Decision-DNNF

Use the `model-enumeration` command:

```bash
decdnnf_rs model-enumeration -i instance.nnf
```

This command has multiple options that affect both the algorithm used to enumerate solutions and the format used to output the model.
For more information on these algorithms and formats, see our research paper *[Leveraging Decision-DNNF Compilation for Enumerating Disjoint Partial Models](https://doi.org/10.24963/kr.2024/48) and the command help (`decdnnf_rs model-enumeration -h`)*.

### Direct access to a model

Use the `direct-access` command:

```bash
decdnnf_rs direct-access --index 5 -i instance.nnf
```

By default, the order of the models is determined by the structure of the formula and may change with new versions of the tool.
Adding the `--lexicographic-order` flag prevents this from happening.
This flag instructs the tool to sort the models in lexicographic order. However, this approach results in longer processing times.

### Uniform sampling

Use the `sampling` command:

```bash
decdnnf_rs sampling -l 5 -i instance.nnf
```

The indices associated with these models function similarly to those for direct access and include the `--lexicographic-order` flag.

## License

Decdnnf-rs is developed at CRIL (Univ. Artois & CNRS).
It is made available under the terms of the GNU GPLv3 license.

## Acknowledgments

Parts of this work has benefited from the support of the AI Chair EXPEKCTATION (ANR-19-CHIA-0005-01) of the French National Research Agency,
including the research papers *[Leveraging Decision-DNNF Compilation for Enumerating Disjoint Partial Models](https://doi.org/10.24963/kr.2024/48)*
and *[Enhancing Query Efficiency for D-DNNF Representations Through Preprocessing](https://doi.org/10.1007/978-3-032-04590-4_9)*.