# ginkou

A Japanese sentence bank. This program can consume sentences from a source,
recognize the words in that sentence, and then setup an index so that sentences
containing a given word can be found. The main use case for this program is to build up
a searchable bank of sentences for learning. When learning it can be very useful to find example
sentences for a given word, which this program enables. This program also does some basic
morphological analysis on a sentence, to be able to store the root word for a verb conjugation.
For example, searching for a verb will also yield sentences containing a conjugated form of that verb.

## Dependencies

This program depends on [mecab](http://taku910.github.io/mecab/) for the aforementioned
morphological splitting. For instructions on installing it, see the [mecab crate](https://github.com/tsurai/mecab-rs).

## Usage

```
USAGE:
    ginkou <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add     Add new sentences to the database.
    get     Search for all sentences containing a given word.
    help    Prints this message or the help of the given subcommand(s)
```

### Adding new sentences

```
USAGE:
    ginkou add [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --database <db>    The database to use.
    -f, --file <file>      The file to read sentences from
```

This will read words from the command line if no file is passed:

```
$ ginkou add
なぜって？
私がきた。
EOF
```

The program can also be used to parse a file containing japanese sentences:

```
ginkou add -f file
```

### Looking up words

```
USAGE:
    ginkou get [OPTIONS] <word>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --database <db>    The database to use.

ARGS:
    <word>    The word to search for in the database.
```

For example, looking up 私 will yield something along the lines of:

```
$ ginkou get 私
私が来た。
```

The output of this will just be matching sentences in an undefined order, seperated by newlines.
This can be piped into programs as you wish, for example to sort the output by line length.
