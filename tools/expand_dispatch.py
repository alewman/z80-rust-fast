#!/usr/bin/env python3
"""Expand grouped dispatch arms into one arm per opcode with literal arguments.

The dispatch tables were written as grouped `match` arms such as

    0x40..=0x75 | 0x77..=0x7F => self.op_ld_r_r(opcode),

which compile to a jump table whose targets share one body, so the handler
sees `opcode` as a runtime value and decodes the register fields with
shifts, masks, and a `match` on every execution. This script rewrites each
grouped arm into one arm per literal with the scrutinee replaced by that
literal in the body:

    0x41 => self.op_ld_r_r(0x41),

so that, once the handler is inlined, the compiler folds the field decode
and the register selection into direct field accesses. The bodies are
otherwise untouched, so the mapping from opcode to handler and argument is
exactly the grouped one.

Usage: tools/expand_dispatch.py <file.rs> <function name> <scrutinee identifier>
Rewrites the file in place. Run once per table; the grouped form stays in
git history for review.
"""
import re
import sys


def split_arms(body):
    """Split a match body into (pattern, arm_text) pairs at top-level arms."""
    arms = []
    depth = 0
    start = 0
    i = 0
    while i < len(body):
        ch = body[i]
        if body.startswith("//", i):
            i = body.find("\n", i)
            if i < 0:
                break
            continue
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
            if depth == 0 and body[i - 1] != "," and _arm_is_block(body, start, i):
                # `} ` closes a block arm; an optional comma follows.
                end = i + 1
                if body[end:end + 1] == ",":
                    end += 1
                arms.append(body[start:end])
                start = end
        elif ch == "," and depth == 0:
            arms.append(body[start:i + 1])
            start = i + 1
        i += 1
    tail = body[start:]
    if tail.strip():
        arms.append(tail)
    return arms


def _arm_is_block(body, start, close):
    """True when the `}` at `close` ends the block that began right after `=>`."""
    arrow = body.find("=>", start)
    if arrow < 0:
        return False
    after = body[arrow + 2:].lstrip()
    if not after.startswith("{"):
        return False
    open_index = arrow + 2 + (len(body[arrow + 2:]) - len(after))
    depth = 0
    for j in range(open_index, close + 1):
        if body[j] in "([{":
            depth += 1
        elif body[j] in ")]}":
            depth -= 1
            if depth == 0:
                return j == close
    return False


def expand_pattern(pattern):
    values = []
    for alt in pattern.split("|"):
        alt = alt.strip()
        if not alt:
            continue
        m = re.fullmatch(r"(0x[0-9A-Fa-f]+)\.\.=(0x[0-9A-Fa-f]+)", alt)
        if m:
            values.extend(range(int(m.group(1), 16), int(m.group(2), 16) + 1))
            continue
        values.append(int(alt, 16))
    return values


def expand(text, function, ident):
    header = re.search(r"\n    (?:pub(?:\(crate\))? )?fn " + re.escape(function) + r"\b", text)
    if not header:
        sys.exit(f"{function}: not found")
    match_start = text.index("match " + ident + " {", header.end())
    body_start = text.index("{", match_start) + 1
    depth = 1
    i = body_start
    while depth:
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
        i += 1
    body_end = i - 1
    body = text[body_start:body_end]

    comments = []
    out = []
    seen = {}
    for arm in split_arms(body):
        # Keep whole-line comments that precede the arm.
        lines = arm.split("\n")
        pre = []
        rest = []
        in_pre = True
        for line in lines:
            if in_pre and (line.strip().startswith("//") or not line.strip()):
                pre.append(line)
            else:
                in_pre = False
                rest.append(line)
        arm_text = "\n".join(rest).strip()
        if not arm_text:
            out.append("\n".join(pre))
            continue
        pattern, arrow, expr = arm_text.partition("=>")
        pattern = pattern.strip()
        expr = expr.strip()
        if expr.endswith(","):
            expr = expr[:-1].rstrip()
        if pattern == "_":
            out.append("\n".join(pre))
            out.append(f"            _ => {expr},")
            continue
        out.append("\n".join(pre))
        for value in expand_pattern(pattern):
            literal = f"0x{value:02X}"
            if value in seen:
                sys.exit(f"{function}: {literal} matched twice")
            seen[value] = True
            new_expr = re.sub(r"\b" + re.escape(ident) + r"\b", literal, expr)
            out.append(f"            {literal} => {new_expr},")
    new_body = "\n".join(line for line in out if line is not None)
    return text[:body_start] + "\n" + new_body + "\n        " + text[body_end:], len(seen)


def main():
    path, function, ident = sys.argv[1:4]
    text = open(path).read()
    text, count = expand(text, function, ident)
    open(path, "w").write(text)
    print(f"{function}: {count} literal arms")


if __name__ == "__main__":
    main()
