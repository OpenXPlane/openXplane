#!/usr/bin/env python3
"""Read only the recovered noise array and clock from the matching Windows build.

No injection, process writes, suspension or execution of recovered functions.
Use 64-bit Python on the Windows PC running the supplied reference executable.
"""
import argparse
import ctypes
import datetime
import hashlib
import json
import math
import struct
import sys
from pathlib import Path

REFERENCE_SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
NOISE_RVA = 0x578f1f0
TIME_RVA = 0x2f01918
TABLE_BYTES = 262144 * 4


def inspect_table(data):
    if len(data) != TABLE_BYTES:
        raise ValueError('Expected exactly 1048576 table bytes')
    values = [v[0] for v in struct.iter_unpack('<f', data)]
    if not all(math.isfinite(v) for v in values):
        raise ValueError('Table contains non-finite float32 values')
    low, high = min(values), max(values)
    return dict(bytes=len(data), entries=len(values), sha256=hashlib.sha256(data).hexdigest(),
                minimum=low, maximum=high, constant=low == high)


def capture(process_id):
    if sys.platform != 'win32' or ctypes.sizeof(ctypes.c_void_p) != 8:
        raise RuntimeError('Capture requires 64-bit Python on Windows')
    from ctypes import wintypes
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    psapi = ctypes.WinDLL('psapi', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.CloseHandle.restype = wintypes.BOOL
    kernel.ReadProcessMemory.argtypes = [wintypes.HANDLE, ctypes.c_void_p, ctypes.c_void_p,
                                       ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
    kernel.ReadProcessMemory.restype = wintypes.BOOL
    psapi.EnumProcessModulesEx.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.HMODULE),
                                         wintypes.DWORD, ctypes.POINTER(wintypes.DWORD), wintypes.DWORD]
    psapi.EnumProcessModulesEx.restype = wintypes.BOOL
    psapi.GetModuleFileNameExW.argtypes = [wintypes.HANDLE, wintypes.HMODULE, wintypes.LPWSTR,
                                        wintypes.DWORD]
    psapi.GetModuleFileNameExW.restype = wintypes.DWORD
    handle = kernel.OpenProcess(0x400 | 0x10, False, process_id)  # QUERY_INFORMATION | VM_READ
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        modules = (wintypes.HMODULE * 1024)()
        needed = wintypes.DWORD()
        if not psapi.EnumProcessModulesEx(handle, modules, ctypes.sizeof(modules), ctypes.byref(needed), 3):
            raise ctypes.WinError(ctypes.get_last_error())
        if needed.value == 0 or needed.value > ctypes.sizeof(modules):
            raise RuntimeError('Incomplete module enumeration')
        module = modules[0]
        path_buffer = ctypes.create_unicode_buffer(32768)
        length = psapi.GetModuleFileNameExW(handle, module, path_buffer, len(path_buffer))
        if not length:
            raise ctypes.WinError(ctypes.get_last_error())
        if length >= len(path_buffer) - 1:
            raise RuntimeError('Executable path truncated')
        executable = Path(path_buffer.value)
        sha = hashlib.sha256(executable.read_bytes()).hexdigest()
        if sha != REFERENCE_SHA256:
            raise RuntimeError('Executable SHA256 does not match the researched build')
        base = int(module)

        def read(rva, size):
            buffer = ctypes.create_string_buffer(size)
            count = ctypes.c_size_t()
            if not kernel.ReadProcessMemory(handle, base + rva, buffer, size, ctypes.byref(count)):
                raise ctypes.WinError(ctypes.get_last_error())
            if count.value != size:
                raise RuntimeError('Partial process memory read')
            return buffer.raw

        if read(0, 2) != b'MZ':
            raise RuntimeError('Main module base does not point to a PE image')
        before = struct.unpack('<d', read(TIME_RVA, 8))[0]
        table = read(NOISE_RVA, TABLE_BYTES)
        second = read(NOISE_RVA, TABLE_BYTES)
        after = struct.unpack('<d', read(TIME_RVA, 8))[0]
        if table != second:
            raise RuntimeError('Noise table changed between reads; retry after loading finishes')
        if not math.isfinite(before) or not math.isfinite(after):
            raise RuntimeError('Non-finite running time snapshot')
        details = inspect_table(table)
        metadata = dict(format='openxplane-runtime-snapshot-v1',
                        captured_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                        executable=str(executable), executable_sha256=sha, process_id=process_id,
                        module_base=hex(base), noise_rva=hex(NOISE_RVA), time_rva=hex(TIME_RVA),
                        running_time_before=before, running_time_after=after,
                        table_stable_across_two_reads=True, table=details,
                        note='Clock and table are separate reads, not an atomic simulation snapshot. '
                             'A stable/finite table does not prove initialization is complete.')
        return table, metadata
    finally:
        kernel.CloseHandle(handle)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--process-id', type=int, help='PID of the running X-Plane.exe')
    parser.add_argument('--output', type=Path, help='New directory for the captured table and metadata')
    parser.add_argument('--inspect-table', type=Path, help='Validate a table file on any OS')
    args = parser.parse_args()
    if args.inspect_table is not None:
        if args.process_id is not None or args.output is not None:
            parser.error('--inspect-table cannot be combined with capture options')
        print(json.dumps(inspect_table(args.inspect_table.read_bytes()), indent=2))
        return
    if args.process_id is None or args.process_id <= 0 or args.output is None:
        parser.error('Capture requires --process-id with a positive PID and --output')
    if args.output.exists():
        parser.error('Output directory already exists; choose a new directory')
    table, metadata = capture(args.process_id)
    args.output.mkdir(parents=True)
    (args.output / 'noise.f32le').write_bytes(table)
    (args.output / 'snapshot.json').write_text(json.dumps(metadata, indent=2), encoding='utf-8')
    print(json.dumps(metadata, indent=2))
    if metadata['table']['constant']:
        print('Captured table is constant; check that the flight has finished loading.', file=sys.stderr)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        print(f'Error: {error}', file=sys.stderr)
        sys.exit(1)
