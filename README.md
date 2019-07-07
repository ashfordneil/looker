Looker
======

A syntactically aware C code search tool.

```
$ cd /path/to/codebase
$ looker build
$ looker search "for (int i = 0;"
./test/vector.c
    for (int i = 0; i < 5; ++i) {
    for (int i = 0; i < 5; ++i) {
    for (int i = 0; i < 5; ++i) {
    for (int i = 0; i < 2; ++i) {
    for (int i = 0; i < 5; ++i) {
    for (int i = 0; i < 2; ++i) {
    for (int i = 0; i < 5; ++i) {
    for (int i = 0; i < 2; ++i) {
    for (int i = 0; i < 5; ++i) {
    for (int i = 0; i < 5; ++i) {
./src/vector.c
        for (int i = 0; i < self->size; ++i) {
```

Looker works in two stages - first it builds an index of the code base you
asked it to search (time consuming) and then it searches that index (quickly).
Both the search query and the indexed code are tokenized using a primitive C
lexer, meaning you don't need to worry about matching whitespace when
searching.
