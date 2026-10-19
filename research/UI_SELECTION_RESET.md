# Whole selection reset caller

Reference: X-Plane.exe12.4.3.11, AMD64 PE base140000000, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
This is disassembly plus completed original-code CPU emulation on synthetic
mapped state, not an observation of the running simulator or rendered UI.

## Recovered14113a460

The subsequent [recursive composition](UI_SELECTION_RECURSIVE_RESET.md) removes
the old catalog range validation and verifies the original signed/wrapping
calculation, including zero, negative and partial distances with mapped backing.

The source first invokes1410fe0f0(owner+38,0x1e1b9,0), then the actual recovered
reset gate14114d330. The gate's default uses actual engine-query1417fb320,
primary comparison14113fff0 and resource getter141a64260. Secondary14113ff10
and memcmp remain explicit controlled services, as in the earlier gate proof.
Memory is handed into and back out of the gate bridge, including when a
service returns an error.

A true gate skips catalog work and notifications, but still clears dirty+18
and tail-calls finish14113d660. Otherwise a fresh engine query is performed.
A zero query byte enters the catalog path directly. A nonzero byte enters it
only if the actual1417fb390 tag query on owner+50 returns true (tags2/3);
other tags go directly to notifications. This distinction is from the original
branches, not a presumed meaning for the engine byte or variant kinds.

The catalog path initializes a32-byte local string A, gets an option/default
string using B, and moves two sequential16-byte words if the returned address
is not A. It resets the returned string but always destroys the live B storage,
regardless of alias. B may be the return address, an independent temporary when
A is returned, or independent of an external returned object. Inline capacity
<=15 requires no free; larger strings use sized free, with the original4096-byte
threshold and aligned-base header rule. Free and producer bodies remain services.

After copy/parse, the signed low32 parsed index is cached. For the verified
nonempty aligned catalog, negative or too-large indices choose **zero**, not
the last record. The fallback scans the original first record's NUL-terminated
name at+58 and requests string assignment. Parsing/copy/assignment ownership
and their internal allocations are not recovered by this caller proof.

The variant getter initializes local V but may return V, an external variant,
or owner+50. The visitor selector is sign-extended **returned** tag+1. After
visiting, destruction instead dispatches on the **live local** tag: only tag0
with its local optional flag set calls cleanup(V+20). The signed -1/0..4 table
entries were inspected in the original; other jump indices are rejected by the
source as unverified. Then the cached index is multiplied by168 and added to
a **fresh** catalog base, read after the visitor and temporary cleanup.
Requested input is forwarded as low32 to141146740. A's live ownership fields
are read for its final free, after that provider can modify them.

The notification path sets+18 to1, traverses live next pointers and reloads the
owner's head after every callback. All paths clear+18 before the final **tail**
request to14113d660. The source does not clear it again after finish: a
controlled finish effect may change it. Native reset has no independent RET;
the harness's finish service returns through the original caller's return
address and requires exact normal completion.

## Local original-code comparison

528 completed calls comprise432 systematic cases and96 deterministic random
cases. The actual gate executes528 times, engine query484, tag query333,
primary comparison39 and indexed getter39. Each original callee's entry and
real RET counts agree per case; both return sites for gate/primary are covered.
327 assignments,200 first-record fallbacks,167 temporary cleanups,386 sized
free requests,817 notifications and528 tail-finish requests execute.

Every provider's identity/arguments/result, all declared input/output memory,
and main-owned stack locals are checked against the port before and after
each request. The stack maps are frame+20..cf plus frame+e0..e7, excluding
callee shadow space and saved registers. Their initial bytes are native input,
and their subsequent bytes must match; provider writes cannot silently stand
in for missing caller stores. The replay inherits production query/gate/
primary/getter implementations, not a recorded boolean shortcut.

Profiles include option/default sources; A/B/external string aliases; inline,
small heap,4095-byte free size and4135-byte aligned free operands; string
fallback and provider changes to A before its final destruction; parsed
negative/valid/too-large and random high-bit indices; low32 requested arguments;
all valid local/returned tags and three variant-return aliases; fresh catalog
base changes by visitor and cleanup; and listener lists with0/1/3 nodes,
head replacement and appending nodes during notification. The controlled
finish checks that dirty was cleared, then changes it to7e, which must survive.
These effects are stress inputs, not claims about the real provider bodies.

The generator verifies the reference digest. Raw numeric corpus stays local:

```sh
set -e
python tools/gen_ui_selection_reset_vectors.py /path/to/X-Plane.exe > /tmp/selection-reset-new.txt
# Rename only after successful completion.
mv /tmp/selection-reset-new.txt /tmp/selection-reset.txt
OPENXPLANE_UI_SELECTION_RESET_VECTORS=/tmp/selection-reset.txt cargo test --offline --locked -p xp-app --test ui_selection_reset_local -- --ignored
```

## Integration and limits

This section records the standalone checkpoint. The subsequent
[factory composition](UI_FACTORY_SELECTION_RESET.md) now compares the actual
reset inside118 whole original factories, with the remaining provider and
viewer limitations stated there.

`ui_dialog_child_selection::Services::reset_checked` is a fallible constructor
boundary, matching the earlier refresh/changed/alternate approach. Its default
still delegates to the legacy controlled reset. The constructor retains its
subsequent explicit finish call. The complete native constructor/factory with
this actual reset is **not yet compared**; existing920 constructor/118 factory
replays continue using the old reset boundary. A shared reset replay supports
checking constructor image, base sentinel and inherited input maps for the
next composition step.

Neither the initial1410fe0f0 nor finish14113d660 body is recovered here. The
former can conditionally invoke finish itself, so two explicit constructor/
reset finish calls do not establish a maximum count. Actual config/default,
copy/parser/string assignment, variant/visitor/cleanup, record assignment,
allocator and notification effects remain dependencies. Resource/catalog/
engine/stack production and ownership/lifetime remain unproven. Logical maps
cover declared bytes, not every allocated byte or every CPU register. Invalid
jump tags, malformed/empty/unbounded catalogs and allocation failures are
outside the verified profile; source errors are not native fault equivalence.

Full dialog/widgets/Main Menu, original Alt/Esc integration, GPU rendering and
paired screenshot/GIF/video acceptance are still incomplete. This CPU caller
proof does not establish whole UI or game parity, and estimates stay unchanged.

Validation:204 ordinary xp-app tests passed,80 opt-in native proof tests ignored
by default. Full reset528 cases and reset-gate/tag2304 cases passed explicitly;
prior920-case constructor and118-case alternate/changed/weather factory replays
passed with their legacy controlled reset. Local Rust/Clippy1.96 workspace/all-
targets with warnings denied passed. Hosted Clippy1.99 had rejected the previous
commit's hex parser for chunks_exact_to_as_chunks; this change uses the suggested
as_chunks API. Hosted validation of this new commit is a separate check.
