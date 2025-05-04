# version-history-inference

Infer a _'constructive'_ version graph/tree for un-versioned files based on diffing file content and apply versioning using this.
This project is for my Cambridge CST Part II Project / Dissertation.

The engine works as follows:

1. Load versions from disk into memory
2. Compare versions using Myers' diffing algorithm to find textual differnces between files
3. Use a heuristic function to generate _'divergence'_ values representing directionally weighted distances
4. Generage a divergence graph for all the versions (pairwise comparison for now)
5. Run Edmonds' algorithm on the graph to build a tree

## Usage

Clone the project:

```
$ git clone https://github.com/JohnGlass97/version-history-inference.git
```

Enter the root directory and install dependencies:

```
$ cd version-history-inference
$ cargo install
```

Run the tool:

```
$ cargo run --release --bin vhi infer <PATH>
```

Or view all commands and their options:

```
$ cargo run --release --bin vhi -- -h
```

The `--` is to separate cargo arguments from those of the CLI tool.

## Other binaries

There are binaries other than `vhi` in `src/bin` related to evaluating the algorithm on GitHub fork trees, unfortunately I haven't had time to write about those here.
