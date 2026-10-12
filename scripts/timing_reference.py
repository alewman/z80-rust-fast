"""Rung 7: every form of instruction, and the reference's instruction_timing for it.

``inputs`` prints one hex byte string per line covering every key space the
lookup distinguishes: each unprefixed opcode, CB xx, all 256 ED xx (defined
and undefined), DD/FD xx, DD/FD CB d xx, and stray prefixes (DD/FD before
DD, FD or ED, one and two deep). ``answers`` reads those lines and prints
each with the reference's answer, in the form z80-timing prints.

Usage: python scripts/timing_reference.py inputs|answers
"""

import sys

from z80_python.timing import instruction_timing

PREFIXES = (0xDD, 0xFD)


def inputs() -> list[bytes]:
    out: list[bytes] = []
    tail = bytes([0x05, 0x00, 0x00, 0x00, 0x00, 0x00])
    for first in range(256):
        if first in (0xCB, 0xED, *PREFIXES):
            continue
        out.append(bytes([first]) + tail)
    for page in (0xCB, 0xED):
        out.extend(bytes([page, op]) + tail for op in range(256))
    singles: list[bytes] = []
    for prefix in PREFIXES:
        for op in range(256):
            if op in (0xCB, 0xDD, 0xED, 0xFD):
                continue
            singles.append(bytes([prefix, op]) + tail)
        for op in range(256):
            for displacement in (0x00, 0x80):
                singles.append(bytes([prefix, 0xCB, displacement, op]) + tail)
    out.extend(singles)
    ed_forms = [bytes([0xED, op]) + tail for op in range(256)]
    for prefix in PREFIXES:
        for rest in singles + ed_forms:
            out.append(bytes([prefix]) + rest)
        for second in PREFIXES:
            for rest in (singles[:300] + ed_forms)[::7]:
                out.append(bytes([prefix, second]) + rest)
    return out


def answer(data: bytes) -> str:
    timing = instruction_timing(data)
    accesses = " ".join(f"{a.kind}@{a.t}" for a in timing.accesses)
    shapes = " ".join(f"{t_states}/{made}" for t_states, made in timing.shapes)
    return f"{data.hex()}\t{accesses} | {shapes}"


def main() -> int:
    command = sys.argv[1] if len(sys.argv) == 2 else ""
    if command == "inputs":
        sys.stdout.write("".join(f"{data.hex()}\n" for data in inputs()))
        return 0
    if command == "answers":
        for line in sys.stdin:
            if line.strip():
                print(answer(bytes.fromhex(line.strip())))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
