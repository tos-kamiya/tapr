# tapr

`tapr` is a table pretty-printer. Outputs a CSV or TSV file as nicely as possible by adjusting column widths.

Screenshot:  
![](docs/images/run1u.png)

## Installation

```sh
cargo install tapr
```

## CLI

```
USAGE:
    tapr [FLAGS] <input>

FLAGS:
    -c, --csv            Force treats input file as CSV
    -h, --help           Prints help information
    -H, --header         Prints first line as a header
    -n, --line-number    Prints line number
    -t, --tsv            Force treats input file as TSV
    -V, --version        Prints version information

ARGS:
    <input>    Input file. Specify `-` to read from the standard input
```

## License

MIT/Apache-2.0
