# Time in the perturbation computation

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). The result of static analysis; the
original was not run.

The global double `0x142f01918`, read by the stall branch `0x141a4190f .. 0x141a419db`, is tied to the name
`sim/time/total_running_time_sec`. The name indicates the units - seconds.

## Verifiable chain

| Element | VA | Content |
|---|---|---|
| Name | `0x14277b690` | `sim/time/total_running_time_sec` |
| Pointer to the name | `0x142fa0260` | `0x14277b690` |
| Pointer to the getter | `0x142fa0280` | `0x1418b6bb0` |
| Pointer to the setter | `0x142fa0288` | `0x1418b6d70` |
| Shared storage | `0x142f01918` | double |

The getter `0x1418b6bb0 .. 0x1418b6bbd` reads the double from the shared storage, performs `cvtpd2ps` and
returns a float32. The setter `0x1418b6d70 .. 0x1418b6d80` converts the input float32 from XMM1 with `cvtss2sd`
and writes the double to the same storage. The size of the table record and its other fields are not
reconstructed here.

The perturbation code reads the internal double directly. So reading through the float dataref and writing back
can change the result: for example, 16777217 seconds is rounded to 16777216 on conversion to float32.

## Implementation

`src/runtime.rs` stores an explicit `RunningTime` snapshot and reproduces the found getter/setter conversions.
`NoiseTable::perturb_at_time` uses the internal double without intermediate rounding. `buffet-eval` takes this
snapshot in seconds and prints the dataref name. The existing `perturb` API is kept. Non-finite snapshots and
writes are rejected by openXplane's diagnostic API; this is an extra check, not established behaviour of the
original.

The module does not advance time automatically. The semantics of updating, pause, acceleration and replay are
not established yet. The write found at `0x1417eacac` adds 0.1 to the input double; this section alone is not
enough to claim a step or a frequency of the simulator clock.

## Reproducing the search

```sh
python3 tools/find_pe_references.py Xplane12/X-Plane.exe --va 0x1418b6d70 --mode qword
python3 tools/find_pe_references.py Xplane12/X-Plane.exe --va 0x142f01918 --mode rip
llvm-objdump -d --start-address=0x1418b6bb0 --stop-address=0x1418b6bbd Xplane12/X-Plane.exe
llvm-objdump -d --start-address=0x1418b6d70 --stop-address=0x1418b6d80 Xplane12/X-Plane.exe
```

The first command on the reference finds the pointer to the setter at file offset `0x2f9e088`; the pointer to
the name is at `0x2f9e060`. The RIP search returns byte-level candidates that must be checked with the
disassembler. The `field_va` field denotes the address of the displacement, not the start of the instruction.
Leaf functions may have no `.pdata` entry; the tool does not attribute the previous function to them.

The initialiser of the noise table at `0x14578f1f0` is still not found. The studied loop `0x1419452d0` works with
a different pointer, `0x1461179e8`, so it is not accepted as its generator.

## Checks

23 unit tests pass; Clippy without warnings; offline build. The new test checks the loss of precision on dataref
read/write, the preservation of the internal snapshot and the refusal of a non-finite write without changing
state. The CLI keeps a snapshot of 16777217 seconds in a double; a synthetic table of ones gives Cl=2.984375 and
Cd=0.498046875 at Cl_in=2 and Cd_in=0.25. Non-finite time is rejected. This is a check of the implementation,
not a comparison with the running original.

The search for the pointer to the setter returns one candidate at the expected address. The RIP search returns
713 candidates; for the setter it does not attribute a false boundary of the previous function. The local
reports `running-time-setter-qword.json`, `running-time-rip.json`, `running-time-cli-check.txt` are in the
Git-excluded directory `research/local/`.
