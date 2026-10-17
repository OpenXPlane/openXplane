# Second dialog child1410d6fc0

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered constructor chain

Outer factory1410d6ca0 calls1410d6fc0 for its second0x2d0-byte allocation, passing
the first returned object as parent. The new port covers the complete outer
constructor through normal return with explicit provider/effect services.

Base table142642d00 and empty120-byte receiver-list sentinel precede global
interface1410c7bb0 at+20 and callback container1410c7b50 at+38. Derived tables
become142643098/1426430c8/1426430e0. Class fields, parent+c8 and containers are
initialized before nested data initializer141a3c750 at+130. That initializer now
runs in the port: nine empty inline strings, scalar defaults, empty containers,
a32-byte sentinel with three self links/word0101 and the empty141a40ad0 clear.
Padding remains unchanged. Nonempty general tree-clear behavior is not claimed.

Property binding140c8f750 receives self+120,1 and literal
`&m_async_apt_filter_done`. Its body remains a service. Engine1417fb320, then
post-query global145a10798 and resource0+6188 choose alternate14111bb90 or normal
selection. Actual provider/effect bodies remain controlled in the comparison.

Normal selection caches catalog length minus1 from the168-byte table before
configured-value140c80d50/14163b530 calls, clamps
the returned signed index to the valid nonempty extent, then **reloads catalog
base and selected record+10 value** after the provider. No textual parsing
behavior is inferred for the opaque configured-value token.

The port executes original-equivalent view/equality and assignment logic:
1417fb3f0/1417f8e10 type0 checks; pointer/value/byte/ordered-float/integer/optional
comparison; equal selections skip change handling. Negative float zero compares
equal to zero; NaN does not. Type0 assignment uses the absent-source optional
path of140e21040, including inline destination optional cleanup. Other valid tags
-1/1/2/3/4 reconstruct type0 through the absent-option140e1ac40 path. Incoming
optional text is absent by constructor code; heap destination optional cleanup
is unsupported, not silently approximated.

Changed selections call14113ba10; alternate and change effects may choose valid
primary+c8 and secondary+d0 parents. Each nonnull parent receives a retained
wrapper copy bound to self+38 with parameters0x1e1b9/0. Both temporaries are
released through the recovered base teardown. Final reset14113a460(arg0) and
update14113d660 are services invoked after registration.

Both selection/context constructors now publish their already initialized base
receiver lists to the shared logical list registry. The kind1 factory adapter
and ordinary selection-then-context test verify that the third child can retain
its0/0 copy in the second child's newly created list, while the second child
retains its0x1e1b9/0 registration with the root. Raw images are construction
snapshots, not executable ABI objects or automatic live native-memory mirrors.

## Whole original replay

`tools/gen_ui_dialog_child_selection_vectors.py` executes original1410d6fc0,
data initializer/empty clear, interface insertion, callback ctor/bind/copy/list,
view/equality/assignment and inline optional cleanup bodies unchanged. Only
property/config/engine/resource/alternate/change/reset/update/allocator effects
are controlled. Synthetic effect inputs do not prove actual provider behavior.

920 cases include720 catalog/index/tag/equality profiles and200 deterministic
random cases. Catalogs contain1/2/5 entries; indices include INT_MIN,-1,0,1 and
INT_MAX. Valid tags-1..4, equality, signed zero, NaN, scalar/optional mismatches,
post-provider catalog replacement, empty/prepopulated parent lists, zero/one/two
parent registrations and global chains0/2 are covered. Actual reached paths:
27 alternate,853 changed and40 equal-selection skips;1201 retained parent copies,
including399 two-parent cases.

Rust matches the entire0x2d0-byte object,120-byte base and32-byte record sentinel,
all raw allocator snapshots/global-head stages, provider-entry/exit snapshots,
selected catalog/global state, interface head/neighbor links, parent/own list
links/counts, logical callback parameters/peers and allocation/free request order.
All920 cases pass. Prior216 controlled-refresh and972 combined actual-refresh
context-child cases pass after receiver-list registry integration.204 ordinary
tests pass;48 opt-in tests are ignored in the ordinary run and local replays run
separately.

Callback-tree heap layout/free-pointer order, actual property/config/effect bodies,
heap optional cleanup, empty/invalid catalogs, invalid tags, allocation failure,
full derived teardown, concurrency and reentrancy remain unverified. Stable
nonempty catalogs/valid parent endpoints and successful allocations are required.
There is no combined original second-plus-third-child/full-factory replay yet;
the ordinary adapter test is not presented as such evidence.

Corpus stays local at `/private/tmp/openxplane-ui-dialog-child-selection.txt`;
opt-in replay uses `OPENXPLANE_UI_DIALOG_CHILD_SELECTION_VECTORS`. Executable,
assets and corpus are not published; source/generator/test/report are published.

## Remaining work

Recover first/fourth child constructors1410d6a40/1410d75a0 and remaining providers,
then compare factory composition with recovered second/third children. Finish
dialog140ec9c10/layout/widget bindings, Main Menu singleton/construction/teardown,
opening ID3/Alt and viewer integration. Original GPU output and paired screenshots/
GIF/video remain missing; Esc still exits the viewer. Overall rubric remains
41.4% implemented/28.2% verified; Rendering/UI verified0. Whole-game parity is unproven.


2026-10-17 update: [first child](UI_DIALOG_CHILD_ROOT.md) now has192 native cases with two controlled refresh bodies. Shared data initializer is reused; source integration connects root/selection/context receivers. Fourth child and root refreshes remain pending; no native combined-factory or visual parity claim.


2026-10-18: [factory with actual root refresh chain](UI_FACTORY_ROOT_REFRESH_CHAIN.md) now has118 complete native calls, all four constructors and actual root/nested/payload bodies. Shared-state execution exposed and fixed the selection resource-index constant (now reloads77/88). Other child/fifth refreshes and full dialog/live GPU remain pending; estimates unchanged.
