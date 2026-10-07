# Datarefs that read a field of the flight object

Static analysis of the getter thunks of the dataref table (`research/DATAREFS.md`). Most getters of the flight model
datarefs have one of two shapes:

* `mov index,%edx; lea 0x1461243b0,%rcx; call 0x141a64250; load OFFSET(%rax) [mul constant]; ret` - the flight object
  `F` of the aircraft index (the same object every ported block of `research/FLIGHT_STEP.md` works on);
* `mov G(%rip),%rax; load OFFSET(%rax) [mul constant]; ret` - a global object, `0x1461020e0` for the cockpit and
  simulation state (986 datarefs).

`tools/extract_flight_datarefs.py` decodes both shapes (loads `movss`/`movsd`/`movl`/`movq`, an optional single
`mulss`/`mulsd` by a constant, such as the radians to degrees factor or the feet to metres factor) and writes the TSV
`crates/xp-dataref/data/flight_datarefs.tsv`: **2388 datarefs**, 1390 on `F`, 986 on `0x1461020e0`, 11 on
`0x1461020e8` and one on `0x142f05d40`. `xp_dataref::flight_map` loads it (`field(name)`, `fields()`): the offset, the
load kind, the scale and whether the dataref is writable. For example `sim/flightmodel/position/theta` is the float at
`F+0x3dc` (the pitch the geographic state block writes), `sim/aircraft/view/acf_livery_index` the int at `F+0x34`.

Not covered: array datarefs, datarefs with several accessors per field, accessors that call other functions
(computed values), and the ones whose getter has a different shape (those are simply absent from the table).
