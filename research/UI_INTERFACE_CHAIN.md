# Global embedded-interface constructor/destructor

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

Ctor `0x1410c7bb0` sets reference interface identity0x1426401f8, newer+8=null,
older+0x10=old global0x14588f558, publishes self as global head and updates old
head's newer link. Concrete main-menu ctor invokes it at panel+0x188, then
replaces the interface table while retaining links. It is not zero initialization.

The first base-table slot points to scalar destructor `0x1410c8d30`, verified
from PE table data. Nearby `0x1410c7bf0` is a different copy constructor, not this
destructor. Destructor restores base identity, updates global head if self is
head, reconnects newer/older neighbors, and requests 24-byte free only when flag
bit0 is set. Higher flag bits do not trigger free. It returns self; its own link
fields remain unchanged. Detached-link images are audit state, not proof that
freed memory remains safely usable in a real runtime.

## Complete native comparison

`tools/gen_ui_interface_chain_vectors.py` executes original ctor/destructor
unchanged; only sized free is controlled. 210 valid sequences cover initial
chains0..4, one/two new interfaces, each head/middle/tail removal position and
flags0/1/2/255/256/257. Existing/final concrete interface identities and all
newer/older links are compared with Rust, along with global head, returned
identity and free request order/head state at the request. All 210 cases pass.
The ordinary regression verifies middle unlink and retained detached links.

Corpus remains local at `/private/tmp/openxplane-ui-interface-chain.txt`; opt-in
comparison uses `OPENXPLANE_UI_INTERFACE_CHAIN_VECTORS`. Original files and
corpus are not published; source/generator/test are published. Rust records
`free_requested` as an audit flag, not an original object field.

## Remaining boundaries

Nodes are stable, construction identities fresh and chains valid. Repeated
invalid destruction, cycles, concurrent mutation, actual deallocation and
higher-level interface callbacks/lifetime are unverified. The full concrete
main-menu destructor dispatches through a different derived thunk; this does
not establish its complete teardown.

Next combine interface linkage with common-panel and embedded-member constructors
for concrete panel initialization, then singleton/dialog/child/layout/resource
creation and actual lifecycle. Opening ID3/Alt, viewer/GPU output and paired
screenshots/GIF/video remain missing. UI verified share stays zero; whole-game
parity is still incomplete.
