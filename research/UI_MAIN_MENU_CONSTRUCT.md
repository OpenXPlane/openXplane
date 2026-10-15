# Concrete Main Menu initialization prefix

Reference X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Executed boundary

`tools/gen_ui_main_menu_construct_vectors.py` executes original instructions from
`0x140ecb340` and asserts RIP is exactly `0x140ecb592`. This is an intermediate
constructor snapshot, **not a normal return or a fully constructed menu**. The
next instruction reads shared-dialog singleton0x145783130; that branch and the
rest of the constructor have not been executed in this comparison.

The original prefix creates the inline identifier `Main Menu`, an empty label
and a translated auxiliary string via key `/_lookup_12_/help/main/`. Translation
is controlled with valid inline capacity15 strings of length0/15. The common
panel constructor runs unchanged, including its input base, callback container,
string copies and four sentinel allocations. Flags168/169/16a are0/0/1.

An optional32-byte callback object captures panel+170; its nested40-byte
sentinel is constructed only on successful allocation. A null32-byte result is
accepted by the native prefix. Embedded interface+188 inserts into the global
chain before concrete tables replace base identities. Three members at+1b0,
+208 and+260 capture their own addresses and append interface pointers to the
parent's registry, growing small vectors as needed. The prefix then initializes
fields2b8..2e8, including raw input byte2c8, null2c9, a32-byte sentinel, all-ones
qword2e0 and float1 at2e8. Unwritten bytes retain their original poison.

## Comparison

480 scenarios combine five valid parent profiles (null; empty; capacity2 with
length1/2; capacity8 with length3), initial global chains of length0/2, three
object poison values, raw input bytes0/1/128/255, two inline translations and
both callback-allocation outcomes. Allocator snapshots include the entire
0x300-byte object and global head at every allocation. Tests compare exact final
object and allocated-block bytes, global/neighbor links, registry storage,
capacities/entries and allocation/copy/sized-free request order. All480 pass.

Only translation, allocator, sized-free and CRT-copy endpoints are controlled;
original constructor/member/registration/growth/callback bodies execute unchanged.
The source composes the existing recovered common-panel, interface-chain and
member logic. Shared callback initialization and member-before-registration
initialization are now reusable. Previous64 input,256 common-panel,180 member
and210 interface-chain native cases pass again after this refactoring. An ordinary
regression also exercises construction with a preexisting interface, all three
registrations and successful/null optional callback allocation.

The numeric corpus remains local at
`/private/tmp/openxplane-ui-main-menu-construct.txt`, replayed through
`OPENXPLANE_UI_MAIN_MENU_CONSTRUCT_VECTORS`. Source, generator and opt-in test
are published; executable bytes, original resources and this corpus are not.

## Remaining work

Recover shared dialogs and their real constructors, remaining children, layout,
resource lookup, heap strings, full teardown and callable lifetimes. Connect the
result to real window creation and the existing keyboard/command graph. Opening
ID3/Alt and rendering original menus remain unverified; Esc still exits the
viewer. There are no paired original/port screenshots or GIF/video. Raw table
addresses are reference identities, not executable host interfaces. This bounded
initialization comparison does not establish whole-game or UI parity; estimates
remain41.4% implemented /28.2% verified overall, UI/Rendering verified0.
