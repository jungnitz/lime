# Notes

## Ambit

MAJ-based

Operations:
* Row copy
* Not
* Destructive Majority

Operands:
* RHS always one or multiple
* LHS depending on what is sensible
* Fixed set of combinations

See:

- [Ambit](https://arxiv.org/pdf/1905.09822)

## COTS DRAM

Operations:
- maj-3 of rows directly adjacent to row decoder [ComputeDRAM]
- 2,4,8,16-ary AND/OR
- not of columns on different sides of sense amplifier

See:

- [ComputeDRAM](https://parallel.princeton.edu/papers/micro19-gao.pdf)
- [FCDRAM](https://arxiv.org/pdf/2402.18736#cite.olgun2021quactrng)

## RRAM / Memristor

### IMPLY

Operations:
- q' = p -> q, p destroyed

Operands:
- Any rows
- 0 / 1

See:
- [IMPLY](https://asic2.group/wp-content/uploads/2017/05/IMPLY-journal-v19.pdf)

### PLiM

Operations:
- Z' = MAJ3(A, ~B, Z)

Operands:
- Any rows
- 0 / 1

See:
- [PLiM](https://si2.epfl.ch/demichel/publications/archive/2016/PEG_DATE16.pdf)

### MAGIC

FELIX is superset I think

### FELIX

Operations:
- n-ary NOR
- 2/3-ary NAND/Minority
- Non destructive with different target row

See:
- [FELIX](https://acsweb.ucsd.edu/~sag076/papers/iccad18_felix.pdf)

### Scouting Logic

Operations:
- AND
- XOR
- OR
- all destructive

Operands:
- any rows
-
