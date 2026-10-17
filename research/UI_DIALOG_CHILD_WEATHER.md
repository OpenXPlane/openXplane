# Fourth dialog child1410d75a0

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The fourth factory allocation is0x3e8 bytes. The recovered source initializes
the receiver and callback container, float wrapper1412a5340, derived table
identities and two equal double-bit defaults4200c388d0000000. Two nested weather
records at+58/+230 use the stores in140e206c0, including padding preservation,
mode/scalar defaults and13 negative-one float fields. The initializer calls
weather141b77b60 with index0/altitude+0 and then restores its sentinel field+28.
The weather calculation itself remains an explicit service.

Parent14111d6a0 supplies raw altitude float bits for a third weather calculation
at+230. The constructor copies all0x1a8 bytes into+58. Parent141135970 returns32
bytes copied into+200. Refresh141139fe0 follows; its body remains controlled.
Bound callback wrappers with arguments0/0 are retained in each parent receiver.
The first receiver uses the saved first argument even if refresh changes+3d8;
the second reloads+3e0 after refresh. Temporaries are destroyed. A late mode7
read at+23c selects special141149360, also a controlled effect.

## Native evidence

720 normal-return whole native constructor runs match source object bytes,
base sentinel bytes, first two allocator-entry snapshots, all provider-entry
snapshots, provider arguments and
order, receiver links/counts, callback peers and temporary allocation/free sizes.
The native raw float/weather initializers, copying, callback/container/list
bodies and temporary teardown execute unchanged. Callback RB backing bytes and
free-pointer order are not claimed; callbacks use the null custom-callable
profile and logical graph representation.

Matrix: three object/allocation poison patterns, four weather mutation profiles,
five raw altitude inputs (+0/-0/1/+infinity/payload NaN), two second-parent
rebindings, two special-mode outcomes and three initial receiver populations
(0/1/3 for first,0/0/2 for second). Weather effects can fill the complete record
with distinct bytes, checking padding and copies without claiming real weather
semantics. Refresh overwrites stored first-parent identity and second-parent
identity, exercising saved/reloaded targets. Incoming second parent also varies.

Controlled bodies: allocation/free, weather141b77b60, altitude14111d6a0,
selection141135970, refresh141139fe0 and special141149360. Successful allocations,
valid stable parents, no concurrent mutation and default FP environment apply.
Raw altitude transfer is tested; numeric weather behavior is not.

Generator: tools/gen_ui_dialog_child_weather_vectors.py with local exe and
Unicorn. Save stdout outside the repo and set
OPENXPLANE_UI_DIALOG_CHILD_WEATHER_VECTORS for cargo test --offline -p xp-app
--test ui_dialog_child_weather_local -- --ignored. No reference corpus, exe or
original assets are published.

## Integration and remaining work

The kind3 factory adapter joins the existing adapters. Ordinary integration now
constructs root, selection, context and weather children, appends weather
callbacks into the actual root/selection receivers and checks temporary removal
and full record copying. Providers are controlled here. This is a source-level
chain, not execution of the complete original factory with all recovered bodies.
Raw constructor snapshots are not automatic mirrors of later logical links.

Next recover root refresh14113a300/141138cf0, weather/provider/effect bodies and
compare composed native factory. Dialog/layout/widget bindings/singleton/menu
and teardown remain incomplete. Alt/Esc in the viewer, original GPU output and
paired screenshots/GIF/video remain unverified. Compatibility estimates stay
41.4% implemented/28.2% verified overall, Rendering/UI verified0.

Validation:720 native cases pass separately;204 ordinary tests pass with50
opt-in tests ignored in that run. The four-child integration also passes after
separating entry addresses from object addresses. Formatting, generator syntax
and diff checks pass. No live viewer/GPU session was tested.
