# Connect 4 solver

Ported to rust from [this](https://github.com/PascalPons/connect4)

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
Searching: alpha -9 beta -8
took: 49.8515ms with 234271 nodes, kn/s: 4699.3
Searching: alpha 10 beta 11
took: 4.7764ms with 32857 nodes, kn/s: 6878.3
Searching: alpha 5 beta 6
took: 101.0962ms with 907167 nodes, kn/s: 8973.3
Searching: alpha -4 beta -3
took: 333.6178ms with 3249028 nodes, kn/s: 9738.8
Searching: alpha 2 beta 3
took: 1.9292213s with 18971888 nodes, kn/s: 9834.0
Searching: alpha -1 beta 0
took: 1.8098913s with 16930931 nodes, kn/s: 9354.7
Searching: alpha 1 beta 2
took: 1.9505654s with 18699356 nodes, kn/s: 9586.6

Score is 2, which means 'x' can win in 19 move(s)

Nodes searched: 59025498
Took 6.1940141s
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

## Plans
- Opening book support is being worked on in the `book` branch.
- Adding some form of multi-threading is planned (probably something like [lazy SMP](https://www.chessprogramming.org/Lazy_SMP)). 
- If you have your own ideas that you want to add, feel free to make a pull request or create an issue.
