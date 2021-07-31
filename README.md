# brainfuck-tools

I'm on my way to writing a compiler with brainfuck output. Therefore I need some tools to know that the code is OK. The first tool, and the most
obvious one to make, is the [interpreter](./interpreter/bfrs), and add a couple options here and there to limit cell size and view the tape after to check
if the results are the same.

Everything was going pretty good. Everything until I tried to implement a `divmod` operation. The program to compile was straightforward, nothing fancy:
```bfs
var a b
set a 7
set b 2
-- a / b -> a
-- a % b -> b
divmod a b a b
```
That program is able to test the capability of the compiler to not only produce a program that computes the correct result, but that also works when any of the targets is the same as the source.

When I compiled the program and ran it, it got stuck in a loop. 7 / 2, nothing crazy here. So that brings the second tool, the [pattern matcher](./tools/bfrs_patterns). This
tool, although currently very primitive, lets me assert that the compiler produced the correct output, abstracting things like the cell addresses, specially for temporaries as those
are really hard to track by hand, and will be even harder when the language gets more features.
