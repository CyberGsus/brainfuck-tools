# bfrs\_patterns

Utility to search for patterns. Currently in development.

Pattern for a move:
  - start at a location, which we'll call `x`.
  - in a loop:
    - decrement 1
    - go to another location which we'll name `y`
    - go back to location `x`
```
x[-y+x]
```
  The first time a match is encountered (for the moment it'll be a memory position),
  an offset from the last position will be recorded for it. The first position is always
  treated as unknown, so the first binding encountered will have an offset from 0.
  Offsets are to the right (`>`).


  This is useful for testing code generators, ensure the code uses a pattern.


- Ideas:
  - multiple patterns:
  ```
  x[y+x-] | x[-y+x]
  ```
  - inside stuff:
  ```
  x[{} z]
  x[{body} z]
  ```
  - reuse patterns:
  ```
  move x y:
    x[-y+x] | x[y+x-]

  x[-t0+y+x](move t0 y)
  ```
