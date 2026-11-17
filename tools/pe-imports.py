#!/usr/bin/env python3
"""The DLLs a Windows executable imports, and a check that none is one it must not need.

    python3 tools/pe-imports.py fcmd.exe            # list them
    python3 tools/pe-imports.py fcmd.exe --static   # fail if the C runtime is a DLL

`fcmd.exe` is built with the C runtime linked in (`.cargo/config.toml`), so it starts on a
Windows without the Visual C++ Redistributable. `--static` is what the release build runs on
the executable it packages: an import of `VCRUNTIME140.dll`, `MSVCP140.dll` or the
`api-ms-win-crt-*` forwarders means the runtime came back as a DLL, and the release stops.
Only the standard library: the PE headers are read by hand.
"""
import struct
import sys

FORBIDDEN = ("vcruntime", "msvcp", "api-ms-win-crt-", "ucrtbase")


def imports(path):
    data = open(path, "rb").read()
    if data[:2] != b"MZ":
        raise ValueError(f"{path}: not an MZ executable")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError(f"{path}: no PE header")
    sections_count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    directories = optional + (112 if magic == 0x20B else 96)
    import_rva = struct.unpack_from("<I", data, directories + 8)[0]
    sections = []
    table = optional + optional_size
    for i in range(sections_count):
        at = table + 40 * i
        vsize, vaddr, rsize, raw = struct.unpack_from("<IIII", data, at + 8)
        sections.append((vaddr, max(vsize, rsize), raw))

    def offset(rva):
        for vaddr, size, raw in sections:
            if vaddr <= rva < vaddr + size:
                return rva - vaddr + raw
        raise ValueError(f"RVA {rva:#x} is in no section")

    names = []
    if import_rva == 0:
        return names
    at = offset(import_rva)
    while True:
        descriptor = struct.unpack_from("<IIIII", data, at)
        if not any(descriptor):
            break
        start = offset(descriptor[3])
        end = data.index(b"\0", start)
        names.append(data[start:end].decode("ascii", "replace"))
        at += 20
    return names


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip().splitlines()[0], file=sys.stderr)
        return 2
    names = imports(argv[1])
    for name in names:
        print(name)
    if "--static" in argv[2:]:
        bad = [n for n in names if n.lower().startswith(FORBIDDEN)]
        if bad:
            print(f"{argv[1]} takes the C runtime from a DLL: {', '.join(bad)}", file=sys.stderr)
            return 1
        print(f"{argv[1]}: the C runtime is linked in")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
