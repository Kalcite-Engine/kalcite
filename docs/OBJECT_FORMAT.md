# `.kco` object format

`.kco` stands for **Kalcite Compiled Object**. It separates language compilation
from platform-specific linking.

## Version 1 header

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | `KCO\0` magic |
| 4 | 2 | little-endian version |
| 6 | 1 | target |
| 7 | 1 | flags |
| 8 | 4 | payload size |
| 12 | 4 | payload FNV-1a checksum |

Initial targets are portable, NumWorks, desktop, and web.

## Planned evolution

Version 1 wraps generated Rust scaffolding to validate the complete pipeline. The
planned sectioned format will contain a symbol table, compact MIR, constant data,
assets, relocations, and a RAM/flash budget manifest.

The target runtime does not interpret `.kco`. The linker/backend transforms it
into machine code before distribution.
