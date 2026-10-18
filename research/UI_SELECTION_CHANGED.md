# Selection change notification and real factory bindings

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The complete original14113ba10 stores byte1 at owner+18, loads the receiver head
from+8, and calls1410c9d10 for each live node+16. After each callback it reloads
the current node's next pointer and the owner's current head. There is no final
dirty-byte store. Thus a callback's dirty-byte changes are preserved; the outcome
is not necessarily1. This differs from the recovered context/weather/fifth refresh
bodies that explicitly clear dirty when finishing.

The body contains no direct owner+d0 second-parent assignment. Notification
callbacks could change that field, but the fresh constructor receiver is empty
in the factory comparison below. ui_selection_changed implements the observed
header/loop behavior on logical mapped memory with a finite traversal guard.
Callback effects remain a Result-returning service; native invalid memory,
nontermination, exceptions, asynchronous mutation and reentrant notifications
are not established by this proof.

## 348 complete original calls

252 systematic cases cross three poison seeds, four initial dirty values, seven
finite list profiles and three callback effects.96 seeded cases add arbitrary
initial bytes. Profiles cover empty/one/three nodes, skipping a node, two forms of
head replacement and appending two new nodes during the first notification.
The new nodes are visited in the same original traversal. Every call reaches the
normal original return, with one real entry/RET observation per case; the change
body is never stubbed. Only1410c9d10 effects are controlled.

All declared object/node/sentinel bytes, ordered wrapper operands and every
callback entry/exit snapshot match the port.547 notification requests occur.
The final dirty byte is1 in154 cases,0 in93 and a5 in101, according to the supplied
callbacks. Other supplied callback changes, including parent words/list counts,
are preserved. These are mapped caller profiles, not real widget ownership or
occurrence frequencies. The void function's incidental return register and
unrelated stack/register state are outside the comparison.

## 118 complete factories and the binding correction

--native-changed implies the existing full weather/root/nested/context/fifth
chain and real indexed resource callers. In95 of118 original factory1410d6ca0
runs, the selection constructor1410d6fc0 now executes actual14113ba10. Its receiver
is freshly self-linked, so there are no notification calls in those95 bodies and
the dirty byte remains1. The other23 cases retain the controlled alternate
selection endpoint.

Earlier cohorts intentionally modeled the opaque changed endpoint with an effect
that could set a second parent at+d0 to stress constructor reloads. The actual
empty-list change body does not make that assignment. Replacing the endpoint now
leaves only11 secondary-parent cases instead of59 and yields601 retained list
entries across the118 factories instead of649.48 synthetic secondary registrations
disappear. These are consequences of the supplied controlled alternate/provider
profiles, not a claim that the live game always has601 bindings. Older649-entry
cohorts retain their explicitly controlled-change scope.

The source constructor calls changed_checked with the actual mutable base
sentinel block. Its legacy default delegates to changed. The new factory adapter
binds ui_selection_changed, checking both the full initial source image and all
supplied sentinel bytes/address before execution and publishing them back after.
The subsequent constructor decisions use the resulting parent word, not a target
copied from the expected final graph. Actual constructor return/allocation/heap/
bundle snapshots, receiver lists and logical callback peers remain compared.
Factory temporary-count checks follow whether the real changed body returned;
the previous modes retain their original second-parent input checks.

## Reproduction and remaining scope

```sh
python tools/gen_ui_selection_changed_vectors.py /path/to/X-Plane.exe > /tmp/selection-changed.txt
OPENXPLANE_UI_SELECTION_CHANGED_VECTORS=/tmp/selection-changed.txt cargo test --offline -p xp-app --test ui_selection_changed_local -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-changed > /tmp/factory-changed.txt
OPENXPLANE_UI_FACTORY_SELECTION_CHANGED_VECTORS=/tmp/factory-changed.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_selection_change_and_weather_chain -- --ignored
```

348 direct and118 composed cases pass.204 ordinary xp-app tests pass with76 opt-in
tests ignored; workspace/all-target Clippy with -D warnings passes. The previous
920 selection-constructor and118 weather-factory replays pass. Regenerating the
previous weather mode preserves all118 C rows byte-for-byte. Formatting, Python
AST and diff checks pass. Executable/assets and generated corpora stay local;
only source, generators and reports are published.

Selection alternate/reset/final, actual notification/effect dispatch1410c9d10,
engine-byte/registry producers and ownership, weather/context/provider computation,
callback RB backing and complete dialog/layout/widget/singleton/menu lifecycle
remain controlled or unfinished. No live viewer/GPU run, original Alt/Esc opening
or paired screenshots/GIF/video evidence was produced. Estimates reviewed and
unchanged:41.4% implemented/28.2% verified overall, Rendering/UI verified0.


The subsequent [whole alternate comparison](UI_SELECTION_ALTERNATE.md) executes
that original body in the remaining23 factory paths. Both exits now compose
with590 retained entries under the new supplied effects. The earlier601/649
controlled-alternate cohorts retain their scope and regenerate unchanged.
