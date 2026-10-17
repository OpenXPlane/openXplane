# First dialog child1410d6a40

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The kind0 factory adapter now initializes the complete0x3e0-byte child. The
source preserves padding, low-byte flag semantics, three receiver lists, the
nested data record, callback container and self-relative inline pointers.
Allocation order is120,32,120,72,72,120,40 bytes. The two tree allocations use
native immediate0x48, which is72 decimal. Their three self links and0101 marker
are initialized while remaining bytes retain allocation poison.

Shared data initializer141a3c750 is now ui_record_construct, used at root+20
and selection+130. Its empty-tree clear is included; general nonempty clear is
not established. Native callback initializer1410c7b50 executes in the reference.

192 normal-return executions of the entire native root constructor match the
port: three initial byte patterns, eight full-width flags, four controlled effect
profiles and two object addresses. Every allocator entry compares the complete
object plus all previously allocated blocks. Each refresh compares target and
complete pre/post object/heap snapshots. Final object, all seven heap blocks,
receiver heads/counts and empty logical callback graph also match.

Only allocation and refresh14113a300/141138cf0 are controlled. Effects deliberately
mutate object and heap bytes. The constructor must clear late fields after the
first refresh and retain second-refresh effects. This does not validate either
refresh body, allocation failures, live listeners, teardown or GPU behavior.
The local corpus is not published; the generator and opt-in replay are.

Reproduction: run tools/gen_ui_dialog_child_root_vectors.py with the local
reference exe and Unicorn/pefile, saving stdout outside the repository. Set
OPENXPLANE_UI_DIALOG_CHILD_ROOT_VECTORS to that file and run cargo test --offline
-p xp-app --test ui_dialog_child_root_local -- --ignored.

The ordinary integration test now constructs root, selection and context children
in sequence through their adapters. Selection retains a callback in the actual
root receiver, and context in the selection receiver. Root refresh effects are
no-ops in this source integration. It is not a native full-factory comparison;
logical list mutations are not automatic mirrors into raw constructor snapshots.
The separate920-case selection replay checks the shared-initializer extraction.

Next: fourth child1410d75a0, both root refresh bodies and remaining providers;
then native composed factory, dialog/layout/bindings/singleton/menu and teardown.
Original Alt/Esc, live GPU and paired screenshots/GIF/video remain unverified.

Validation: 204 ordinary tests pass; 49 opt-in tests are ignored in that run.
The192-case root and920-case selection replays pass separately. Formatting,
Python syntax and git diff checks also pass. No live GPU check was performed.


2026-10-17: [fourth child](UI_DIALOG_CHILD_WEATHER.md) now has720 native comparisons with controlled providers; source integration connects all four adapters. Root refreshes and provider bodies still need recovery, followed by native full-factory and visual checks.
