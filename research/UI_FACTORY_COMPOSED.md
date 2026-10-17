# Entire child factory with four original constructor bodies

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The earlier576-case factory test replaced each child constructor. This new local
corpus executes factory1410d6ca0, root1410d6a40, selection1410d6fc0,
context1410d7490 and weather1410d75a0 in one original call, plus original inline
fifth-object initialization and hash bucket resize. Final RAX equals bundle
identity and execution reaches the normal return sentinel. All allocations
succeed. The216 cases are a different scope from the earlier allocation-null
matrix, which is retained and still passes.

## Shared state

The Rust factory dispatches each kind to its existing recovered constructor
adapter. Returned base/record blocks stay in factory ownership. The new
publish_callback_links bridge commits logical list next/previous fields and
counts, plus container peer counts, to mapped child/receiver images after each
constructor returns. Retained receiver entries have mapped16-byte links; their
wrapper payload and native callback RB nodes remain logical. Earlier lists and
containers survive subsequent constructors and receive their retained copies.

Native snapshots at every outer allocation and every child return compare the
whole bundle and declared heap. Full child objects, six raw root allocations,
selection base/record sentinels, context/weather base sentinels, fifth receiver/
record/hash buckets and retained-entry links are compared. Each constructor's
initial allocation event compares its full object image before allocation;
controlled leaf entries compare object images/arguments and apply explicitly
recorded provider effects. Final list sentinel/order/count and callback peers/
wrapper arguments match across all seven receivers and four callback containers.
Selection's global interface head is checked; its preexisting chain is empty.

## Matrix and observed coverage

216 cases:3 allocation/bundle poison seeds ×6 selection tags0/1/2/3/4/255 ×2
engine gates ×2 secondary callback targets ×3 signed indices (INT_MIN/0/INT_MAX)
against a stable three-entry catalog. Index clamping and the alternate selection
branch execute in the same factory run. Weather profiles vary provider writes,
altitude raw float bits including signed zero/infinity/NaN, post-refresh second
parent rebinding and special mode7. Context refresh effects vary raw payload bits.

864 actual child constructions and216 fifth objects create3024 constructor
internal allocations and1728 outer/inline-fifth allocations.972 retained entries
survive temporary wrapper destruction: selection retains one or two, context
retains one and weather retains two. The selection receiver is shared by context
and weather; weather's second target can instead be the context receiver.
Selection's optional secondary target is the root's nested receiver. This tests
cross-child lifetime and receiver publication that independent constructor
fixtures could not establish.

## Boundaries and reproduction

Root nested/final refresh, context refresh, selection property/engine/resource/
configuration/change/reset/finish, weather population/altitude/selection/refresh/
special and fifth refresh remain controlled providers in this corpus. Original
callback bind/copy/list/temporary teardown and data/variant/hash initialization
execute unchanged. Callback RB heap layout/free-pointer order, allocation failure
inside constructors, exception cleanup, preexisting interface chains and live
renderer behavior are not established. The earlier576 factory cases separately
cover outer null allocations with child bodies controlled.

The prior118-case whole-root-constructor test executes its actual two nested
refreshes, final root refresh and payload helpers; that is separate evidence.
Do not infer that those bodies execute inside this216-case factory corpus.

```sh
python tools/gen_ui_factory_composed_vectors.py /path/to/X-Plane.exe > /tmp/factory-composed.txt
OPENXPLANE_UI_FACTORY_COMPOSED_VECTORS=/tmp/factory-composed.txt cargo test --offline -p xp-app --test ui_factory_composed_local -- --ignored
```

Only the generator/source/report are published. Corpus, executable and original
assets stay local. Next compose actual recovered refreshes into this shared
factory and recover fifth refresh141139460/remaining providers, full dialog,
layout/widget bindings/singleton/Main Menu and teardown. Live Alt/Esc, GPU pixels
and paired screenshots/GIF/video remain unverified. Compatibility estimates were
reviewed and stay41.4% implemented/28.2% verified overall; Rendering/UI verified0.

Validation:204 ordinary tests pass;58 opt-in tests are ignored in the ordinary
run. The216-case composed replay and earlier576-case controlled-child factory
replay both pass explicitly. Formatting, Python syntax and diff checks pass.
No live viewer/GPU run or paired visual capture was performed.
