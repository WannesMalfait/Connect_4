# Connect 4 solver

Ported to rust from [this](https://github.com/PascalPons/connect4)

The goal of this program is to be able to determine whether a given connect 4 position is winning, lost or a draw, and in how many moves the result can be achieved. It makes use of a bitboard representation to be able to check for alignments efficiently using bitwise operations. 

## Basic usage

You can get information about the program using
```
> help
```

To set up a position you can use `position` which will play the given moves from the starting position:
```
> position 4 4 5
```
To make some moves from the current position you can use `moves/play/move`
```
> play 7 5 1 2
```

To get a score for the current position use `solve`, to get scores for all the columns use `analyze`.
```
> play 4 4 5
Played columns: [4, 4, 5]

Current position:
.......
.......
.......
.......
...x...
...oo..

> solve
Searching: alpha -9 beta -8 [min -19, max 20]
Took: 30.753895ms, total nodes 234400, kn/s: 7813
pv: 3 6 7 4 4 
Searching: alpha 10 beta 11 [min -8, max 20]
Took: 3.227257ms, total nodes 264222, kn/s: 8006
pv: 3 6 7 4 4 5 5 6 4 4 3 3 3 5 3 3 5 5 2 2 
Searching: alpha 5 beta 6 [min -8, max 10]
Took: 108.404003ms, total nodes 1139648, kn/s: 8025
pv: 3 6 7 4 4 5 5 6 4 4 3 5 5 5 2 3 3 3 2 3 2 2 6 6 1 1 1 
Searching: alpha -4 beta -3 [min -8, max 5]
Took: 399.142056ms, total nodes 4393337, kn/s: 8120
pv: 3 6 7 4 
Searching: alpha 2 beta 3 [min -3, max 5]
Took: 2.058254278s, total nodes 21736819, kn/s: 8363
pv: 3 6 7 3 4 4 4 3 
Searching: alpha -1 beta 0 [min -3, max 2]
Took: 2.111754726s, total nodes 39002773, kn/s: 8279
pv: 3 6 7 
Searching: alpha 1 beta 2 [min 0, max 2]
Took: 2.39107231s, total nodes 58487426, kn/s: 8235
pv: 3 6 7 3 

Score is 2, which means 'x' can win in 19 move(s)
Total number of nodes: 58487426
Took 7.102748709s
```

### Benchmark

To test the performance of the solver you can run a benchmark on one of the test files. The bench is run using the strong or weak solver depending on the current setting. The weak solver only calculates whether the position is a win, draw or loss, which makes it faster. You can toggle it using `toggle-weak`:
```
> toggle-weak
Weak set to true

> toggle-weak
Weak set to false

> bench ./benchmark_files/begin_easy
```
You can specify the maximum number of lines to solve if you want a faster result. Use "all" instead of a file path to run all the benchmarks. This searches for benchmarks in `./benchmark_files/`.
```
bench all 100
```

### Opening Books

In the starting position, it can take a long time to find the best move. For this reason an opening book can be loaded, which knows the best moves in starting positions. An opening book is a file where each line is an entry with three values.
```
position_key best_move score
```

By default, the program looks for a book `./opening_book.book`, but a custom path can be specified:
```
> load-book ./my_awesome_openings.book
```

The `generate-book` command can be used to generate a book from the current position to a given depth (this can take a long time!):
```
> generate-book 3
```

### Multiple Threads
The number of threads can be set using the `threads` command. At the moment, the searcher only benefits from having 2 threads instead of 1. Adding more threads will only slow down the searcher. Utilizing multiple threads more effectively is part of the future plans. The idea is to do a "P-ary" search for the score of the position, instead of the current binary search.

## Plans
- Improve the multithreaded search.
- Use `Clap` to do the argument parsing instead of doing everything manually. This should make the help messages better, and make the code more maintainable.
- Improve the opening book generation. At the moment it doesn't work properly if a (partial) opening book is already loaded. Ideally it should just expand the existing opening book, to the requested depth.
- If you have your own ideas that you want to add, feel free to make a pull request or create an issue.
