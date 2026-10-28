#!/usr/bin/env python3
"""Benchmark complete CLI invocations: reference Python memo vs native memo.

  bench.py <work-dir> [sizes...]     default sizes: 1000 100000 1000000

Builds one synthetic store per size (LOG.txt plus every tree level, so wake
needs no naps), checks that both implementations print the same output for
every read-only command, then reports the median wall time and the peak
resident memory of each. Measurements are warm-cache: each command runs once
untimed first. Results go to stdout as a Markdown table.
"""

import os
import shutil
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PY = os.path.join(ROOT, "reference", "memo.py")
RS = os.path.join(ROOT, "target", "release", "memo")
RUNS = 5

QUERIES = [
    ("recall elixir", ["recall", "elixir"]),
    ("recall absent", ["recall", "zqxjkvbw"]),
    ("recall .", ["recall", "."]),
    ("recall unicode", ["recall", "ünïcödé"]),
    ("recall alternation", ["recall", "topic-(1|22|33) "]),
    ("wake", ["wake"]),
    ("nap (read)", ["nap"]),
]


def build(path, n):
    """A store of n memories with every tree block built."""
    if os.path.exists(os.path.join(path, "LOG.txt")):
        return
    os.makedirs(os.path.join(path, "TREE"), exist_ok=True)
    with open(os.path.join(path, "LOG.txt"), "wb") as f:
        buf = []
        for i in range(n):
            extra = " elixir" if i % 10 == 0 else ""
            extra += " Ünïcödé" if i % 1000 == 0 else ""
            t = "#%d 2026-01-01 synthetic memory %d about topic-%d%s" % (i, i, i % 97, extra)
            b = t.encode()
            buf.append(b + b" " * (319 - len(b)) + b"\n")
            if len(buf) >= 65536:
                f.write(b"".join(buf))
                buf = []
        f.write(b"".join(buf))
    size = 2
    while size <= n:
        with open(os.path.join(path, "TREE", str(size)), "wb") as f:
            buf = []
            for k in range(n // size):
                t = "summary of %d-%d" % (k * size, (k + 1) * size - 1)
                b = t.encode()
                buf.append(b + b" " * (287 - len(b)) + b"\n")
            f.write(b"".join(buf))
        size *= 2


def pretty(p):
    home = os.path.expanduser("~")
    return "~" + p[len(home):] if p.startswith(home + os.sep) else p


def run(exe, args, store, timed=True):
    env = dict(os.environ, MEMORY_DIR=store)
    cmd = ["/usr/bin/time", "-l" if sys.platform == "darwin" else "-v", exe] + args
    t0 = time.perf_counter()
    p = subprocess.run(cmd, env=env, capture_output=True)
    wall = time.perf_counter() - t0
    err = p.stderr.decode(errors="replace")
    rss = 0
    for line in err.splitlines():
        if "maximum resident set size" in line:
            rss = int(line.split()[0])  # bytes on macOS
        elif "Maximum resident set size" in line:
            rss = int(line.split()[-1]) * 1024  # KiB on Linux
    out = p.stdout.decode().replace(pretty(exe), "<ME>")
    return wall, rss, p.returncode, out


def measure(exe, args, store):
    run(exe, args, store)  # warm up
    walls, rsss = [], []
    for _ in range(RUNS):
        w, r, _, _ = run(exe, args, store)
        walls.append(w)
        rsss.append(r)
    return statistics.median(walls), max(rsss)


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    work = sys.argv[1]
    sizes = [int(s) for s in sys.argv[2:]] or [1000, 100000, 1000000]
    if not os.path.exists(RS):
        sys.exit("build first: cargo build --release")
    print("| Records | Command | Python | Native | Speedup | Python RSS | Native RSS |")
    print("| ---: | --- | ---: | ---: | ---: | ---: | ---: |")
    for n in sizes:
        store = os.path.join(work, "store-%d" % n)
        build(store, n)
        for name, args in QUERIES:
            a = run(PY, args, store)
            b = run(RS, args, store)
            if (a[2], a[3]) != (b[2], b[3]):
                sys.exit("OUTPUT DIFFERS at %d records for %s" % (n, name))
            pw, pr = measure(PY, args, store)
            rw, rr = measure(RS, args, store)
            print("| %s | `%s` | %.1f ms | %.1f ms | %.1fx | %.1f MB | %.1f MB |"
                  % (format(n, ","), " ".join(args), pw * 1e3, rw * 1e3, pw / rw,
                     pr / 1e6, rr / 1e6))
            sys.stdout.flush()
        # note fsyncs, so it runs on a throwaway copy of the smallest store.
    scratch = os.path.join(work, "note-store")
    shutil.rmtree(scratch, ignore_errors=True)
    os.makedirs(os.path.join(scratch, "TREE"))
    open(os.path.join(scratch, "LOG.txt"), "w").close()
    for label, exe in (("Python", PY), ("Native", RS)):
        walls = [run(exe, ["note", "benchmark note %d" % i], scratch)[0] for i in range(RUNS * 2)]
        print("\n`note` (with fsync), %s: median %.1f ms" % (label, statistics.median(walls) * 1e3))


if __name__ == "__main__":
    main()
