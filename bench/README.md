# Secondary bench workloads

ZEXALL (`scripts/bench.sh`) is the number every commit reports, but its
instruction mix barely touches some paths. These `flat`-host manifests run
one tight loop each for 300,000,000 instructions so a change to such a path
can be measured on its own:

```text
target/release/z80-bench bench/ix-loop.json    # DD-prefixed: LD A,(IX+d), ADD A,(IX+d), LD (IX+d),A, INC IX, DEC IX, JR
target/release/z80-bench bench/ed-loop.json    # ED-prefixed: NEG, ADC HL,BC, LD A,I, LD A,R, JR
```
