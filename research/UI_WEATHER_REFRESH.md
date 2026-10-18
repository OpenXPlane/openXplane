# Whole weather-child refresh and factory composition

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

ui_weather_refresh recovers the complete outer141139fe0 body. It short-circuits
the two initial flag queries, then either initializes the working record or
repeats engine/global/resource gating. The accepted-resource path initializes a
scratch record, obtains/applies weather data and loads two raw64-bit fields from
settings+390/+398. Each settings pointer is cached before its override query;
the second getter reloads the live global index. A nonzero low return byte
selects a positive-zero bit pattern rather than the settings word.

The default path gets a manager and raw XMM altitude bits, requests a weather
record, copies424 bytes into the work record, applies priority mode7/mode3 flags
and obtains16 context bytes through a freshly loaded parent identity. The final
work-to-display copy uses offsets230->58 and is disjoint. The owner is marked
dirty, listeners are notified with live post-callback next/head reloads, and the
dirty byte is cleared at completion.

Provider-result copying preserves the original forward16-byte load/store order,
including partial overlap; substituting a whole-buffer memmove would differ.
The settings words are copied as bits, with no host floating arithmetic. This
preserves negative zero, infinities and NaN payloads without claiming numerical
weather computation. Function arguments/results in floating ABI positions are
represented as raw low-word XMM bits by the provider interface.

## 384 complete original executions

288 systematic plus96 seeded profiles execute the whole original function to
its normal return. Engine query1417fb320 and indexed getters141a64260/141a64250
execute originally; per-case entry/real-RET counts agree.117 engine-query,31
resource and22 settings returns occur.11 profiles reach the accepted-resource
path,106 default weather requests occur, and267 take an initial flag/reset path.
All four pairs of settings zero-override results are reached.

Whole object, declared mapped blocks, active global/index and every provider
entry/exit snapshot match. Ordered logical operands, scratch input reads and
local writes are checked. Initial scratch data is supplied explicitly; saved
register spills/unused padding and the void function's incidental return register
are not compared. Original normal RIP return is asserted.

Default record sources cover25 scratch-stack,29 external,27 exact-self and25
partially overlapping owner views. The latter are caller-only mapped raw profiles,
not proof of complete valid widget ownership. Mutable override providers change
settings fields after getter return and shift the index between queries, checking
cached pointer use and the next fresh lookup. Finite empty/stable/mutated listener
profiles issue575 notifications, including head replacement and next-node skips.
Callbacks may change the dirty byte; the outer function still clears it.

## 118 complete factory executions

--native-weather includes the actual root/nested/payload/context/fifth chains,
all previously recovered resource callers and the actual weather refresh inside
constructor1410d75a0. Weather executes39 actual engine-query,18 resource and22
settings returns. The118 calls divide into79 flag/reset,11 accepted-resource and
28 default weather branches. Default sources are9 stack,9 external and10 exact-
self records. Fresh weather receiver lists are empty here; standalone cases
provide the nonempty listener evidence.

The source bridge checks the initial full weather image, inherited active/index
and all20 root-derived resource/settings/descriptor input regions. Weather query
byte and settings+390/+398 words are supplied at weather entry as new fixture
inputs; their real producers/lifetimes are not established. A controlled weather
provider may rebind the second constructor parent. The original constructor's
subsequent reload and retained registrations match, along with all existing
child-return/heap/bundle snapshots and649 retained list entries.

ui_dialog_child_weather now calls refresh_checked, whose default preserves the
legacy refresh service. A host can bind the recovered Result-returning image
bridge and propagate errors before registering callbacks. The factory replay
updates global/index/table state after this bridge, then compares inherited
inputs again in fifth refresh. Existing replay stages now use an explicit
ReplayMode enum rather than a growing boolean argument list.

## Boundaries and reproduction

Weather calculation/data initialization140e206c0/141b75880/1410d3a20, manager/
selection/weather/context providers, CLI/override query bodies and notification
effects remain controlled. Scalar records and pointers are synthetic inputs.
Actual registry/engine-byte/settings production, ownership, allocation/exception
failure behavior, asynchronous mutation and nonterminating/invalid graphs are
outside these comparisons. Listener traversal is bounded in the source model.
Selection alternate/changed/reset/final bodies and remaining dialog/widget/layout/
singleton/menu teardown still need recovery; this is not a full UI or weather
engine implementation.

```sh
python tools/gen_ui_weather_refresh_vectors.py /path/to/X-Plane.exe > /tmp/weather-refresh.txt
OPENXPLANE_UI_WEATHER_REFRESH_VECTORS=/tmp/weather-refresh.txt cargo test --offline -p xp-app --test ui_weather_refresh_local -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-weather > /tmp/factory-weather.txt
OPENXPLANE_UI_FACTORY_WEATHER_CHAIN_VECTORS=/tmp/factory-weather.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_weather_refresh_and_all_record_tables -- --ignored
```

All384 direct and118 composed cases pass.204 ordinary xp-app tests pass with74
opt-in tests ignored. The previous720 weather-constructor and118 child-table
factory replays pass; regeneration preserves all118 previous factory C rows
byte-for-byte. Workspace/all-target Clippy with -D warnings, formatting, Python
AST and diff checks pass. Original executable/assets and generated corpora remain
local; only source, generators and reports are published.

No live viewer/GPU run or original Alt/Esc/paired screenshots/GIF/video evidence
was produced. Estimates reviewed and unchanged:41.4% implemented/28.2% verified
overall, Rendering/UI verified0. Complete the remaining providers/selection and
dialog construction, connect actual menu state/resources to live rendering and
obtain paired captures before claiming graphical parity.


The later [actual selection-change composition](UI_SELECTION_CHANGED.md) replaces
that opaque endpoint in95 reached constructor paths and matches601 retained
entries over118 factories. The earlier649-entry cohorts above keep their
controlled-change scope and regenerate unchanged.
