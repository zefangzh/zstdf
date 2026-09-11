"""Generate only synthetic STDF evidence, then render with the local CLI.

The example stages do not represent any product's required manufacturing flow.
Run from any directory: python generate_demo.py --cli <zstdf-cli executable>
"""
import argparse
import gzip
import struct
import subprocess
from pathlib import Path


def record(typ, sub, body):
    return struct.pack("<HBB", len(body), typ, sub) + body


def cn(value):
    value = value.encode("utf-8")
    return bytes([len(value)]) + value


def run(job, lot, time, devices, *, wafer="W1", duration=10):
    data = record(0, 10, bytes([2, 4]))
    mir = struct.pack("<IIBBBBHB", time, time, 1, 80, 32, 32, 0, 32)
    for value in [lot, "SYNTHETIC", "DEMO_NODE", "DEMO_TESTER", job, "1"]:
        mir += cn(value)
    data += record(1, 10, mir)
    data += record(1, 80, bytes([1, 1, 2, 1, 2]) + b"".join(cn(v) for v in ["handler", "H1", "probe-card", "PC1", "loadboard", "LB1", "DIB", "DIB1"]))
    data += record(2, 10, bytes([1, 1]) + struct.pack("<I", time) + cn(wafer))
    for i, device in enumerate(devices):
        x, y, flag, hard_bin, soft_bin, fallback = device
        site = i % 2 + 1
        data += record(5, 10, bytes([1, site]))
        if fallback:
            for test, name, value in [(1, "dieCoordinateX", x), (2, "dieCoordinateY", y)]:
                data += record(15, 10, struct.pack("<IBBBBf", test, 1, site, 0, 0, value) + cn(name))
        px, py = (-32768, -32768) if fallback else (x, y)
        data += record(5, 20, struct.pack("<BBBHHHhhI", 1, site, flag, 2 if fallback else 0, hard_bin, soft_bin, px, py, 5) + cn("REPEATED_PART_ID"))
    data += record(1, 30, bytes([1, 255]) + struct.pack("<IIIII", len(devices), 0, 0, sum(d[2] == 0 for d in devices), 0))
    data += record(1, 20, struct.pack("<I", time + duration))
    return data


def device(x, flag=0, hb=1, sb=1, fallback=False):
    return (x, 1, flag, hb, sb, fallback)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, default=Path(__file__).resolve().parent / "generated")
    args = parser.parse_args()
    root = args.output_dir.resolve()
    inputs = root / "inputs"
    inputs.mkdir(parents=True, exist_ok=True)
    # Every name is owned by this generator; unrelated files are never removed.
    fixtures = {
        "01-a.stdf": run("DEMO_A", "DEMO_CLOSED", 100, [device(1), device(1, 8), device(1), device(2), device(3), device(4, 8), device(5), device(7, 16), device(9)]),
        "02-b.stdf": run("DEMO_B", "DEMO_CLOSED", 200, [device(1), device(5), device(5, hb=2, sb=7)]),
        "03-c.stdf": run("DEMO_C", "DEMO_CLOSED", 300, [device(1), device(2), device(5), device(7)]),
        "04-open.stdf": run("DEMO_A", "DEMO_OPEN", 100, [device(6, 8), device(6)]),
        "05-fallback.stdf": run("DEMO_B", "DEMO_UNLINKED", 400, [device(8, fallback=True)], wafer="bad!"),
        "06-mapped.stdf": run("DEMO_B", "DEMO_MAPPED", 500, [device(1, fallback=True)], wafer="bad!"),
        "07-unmatched.stdf": run("UNMATCHED_PROGRAM", "DEMO_CLOSED", 500, [device(9)]),
        "08-overlap.stdf": run("DEMO_A", "DEMO_OPEN", 100, [device(10)]),
        "09-overlap.stdf": run("DEMO_A", "DEMO_OPEN", 105, [device(10, 8)]),
    }
    for name, content in fixtures.items():
        (inputs / name).write_bytes(content)
    (inputs / "01-a-copy.std.gz").write_bytes(gzip.compress(fixtures["01-a.stdf"], mtime=0))
    config = Path(__file__).resolve().parent
    subprocess.run([str(args.cli.resolve()), "traceability", str(inputs),
                    "--flow-config", str(config / "flow.json"),
                    "--flow-closures", str(config / "closures.json"),
                    "--identity-map", str(config / "identities.json"),
                    "--output", str(root / "demo.html")], check=True)
    print(root / "demo.html")


if __name__ == "__main__":
    main()
