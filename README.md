# Connect 4 solver

Ported to rust from [this](https://github.com/PascalPons/connect4)

## Basic usage

You can get a list of commands using:
```
commands
```
or `help`. 

To set up a position you can use `position` which will play the given moves from the starting position:
```
position 4 4 5
```
To make some moves from the current position you can use `moves/play/move`
```
play 7 5 1 2
```

To get a score for the current position use `solve`, to get scores for all the columns use `analyze`.
```
play 4 4 5
Played columns: [4, 4, 5]
.......
.......
.......
.......
...x...
...oo..
solve
Searching: alpha -9 beta -8
took: 27.9229ms with 234441 nodes, kn/s: 8395.9
Searching: alpha 10 beta 11
took: 3.3803ms with 32860 nodes, kn/s: 9719.9
Searching: alpha 5 beta 6
took: 93.0796ms with 908946 nodes, kn/s: 9765.2
Searching: alpha -4 beta -3
took: 327.358ms with 3266571 nodes, kn/s: 9978.6
Searching: alpha 2 beta 3
took: 1.9252398s with 19116307 nodes, kn/s: 9929.3
Searching: alpha -1 beta 0
took: 1.80603s with 18270199 nodes, kn/s: 10116.2
Searching: alpha 1 beta 2
took: 2.2257773s with 23581023 nodes, kn/s: 10594.5
Score: 2, which means 'x' can win in 19 move(s)
Nodes searched: 65410347
Took 6.4156825s
```

### Benchmark

To test the performance of the solver you can run a benchmark on one of the testfiles. The bench is run using the strong or weak solver depending on the current setting. You can toggle it using `toggle-weak`:
```
toggle-weak
Weak set to true
toggle-weak
Weak set to false
bench ./benchmark_files/begin_easy
```
You can specify the maximum number of lines to solve if you want a faster result. You can also run the benchmarks for all the files.
```
bench all 100
```