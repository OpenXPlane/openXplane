# Actual class membership and Favorites delegation

Reference X-Plane.exe12.4.3.11 SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Static evidence: LLVM disassembly141a400f0..141a40156 (102 bytes, including
both normal tree paths and the Favorites tail transfer). Execution evidence:
3312 complete original calls under Unicorn2.1.4. The port is
`ui_record_class_membership::contains`.

The function consumes low32 of the index. Index11 calls singleton1414cc7f0,
then tail-transfers to1414cc820 with that returned pointer and the cached
original record address. It does not read the record's tree in this branch.
The handler's entire RAX and memory effects are returned unchanged; low-byte
truth normalization would lose observable output. Both providers are controlled
in the current oracle. Their names/implementation/registry lifetime are not
established by this delegation proof. Favorites is the corresponding verified
label at index11, not a claim about the final visible UI.

Other indices read head at record+180 and root at head+8. Nonzero byte+19 is
terminal; signed DWORD+1c comparisons choose left0 or right10 while retaining a
lower-bound candidate. Final nil and signed candidate-key checks determine the
result. The normal branches write AL only: high56 RAX bits retain the final
terminal-node address. The source preserves those bits rather than returning
canonical0/1. High32 index aliases and signed boundaries are retained.

## Original comparison evidence

The3312 cases combine8 masks,3 ordered tree shapes,2 nonzero nil markers,
23 signed/unsigned/high32 index variants and3 poison patterns. They include
physical empty roots, key11 present/absent, full/mixed sets, MIN/-1/12/MAX
boundary keys, balanced trees and one-sided chains. Every original call reaches
normal RET; the Favorites tail uses the real caller's epilogue/transfer and
controlled final handler return. Its288 cases reach288 singleton requests and
288 handler requests. Tree paths have no controlled leaves.

Strict Rust replay matches every full64-bit RAX, every declared non-stack byte
and provider function/arguments/pre-state/post-state. Singleton effects replace
the live tree pointer and alter singleton storage; the tail still passes the
cached original receiver and newly returned singleton pointer. Handler effects
write live record bytes, and arbitrary64-bit handler results survive. All3312
cases pass; both the local proof and ordinary Favorites test complete in1.62s.

The ordinary test supplies no tree or record mapping at all and calls the high32
alias0x10000000b. It verifies two provider requests, shared live effects and
full result0xfedcba9876543200, whose low byte is zero. This protects the source's
delegation behaviour in hosted CI without redistributing the original.

## Reproduction and limits

```sh
set -e
python tools/gen_ui_record_class_membership_vectors.py /path/to/X-Plane.exe > /tmp/membership-new.txt
mv /tmp/membership-new.txt /tmp/membership.txt
OPENXPLANE_UI_RECORD_CLASS_MEMBERSHIP_VECTORS=/tmp/membership.txt cargo test --offline --locked -p xp-app --test ui_record_class_membership_local -- --include-ignored
```

The generator checks reference SHA256 and normal return/instruction bound.
The local raw corpus contains23116052 bytes, SHA256
`ae53136d72c60fdbbc36be59ce7b6616bae671a5fe4c80e82ffcf8434fae7bf2`.
Its412573-byte gzip archive was fully decompressed and hash-checked.
Numeric corpus/executable remain local; hosted CI runs the ordinary test only.
Source rejects cyclic searches after100000 visits. Malformed trees, native
faults, exception/unwind/register-save ABI, true tree production/balancing,
arbitrary ownership/lifetime and actual singleton/Favorites bodies are unproved.
The function is available for the next secondary14113ff10 reconstruction;
that caller is not upgraded by this isolated proof. Existing class-string
builders retain their own directly recovered ordered search sequence.
Whole dialog/layout/widgets/Main Menu, Alt/Esc live viewer/GPU and paired
screenshots/GIF/video remain unverified. Estimates reviewed and unchanged.
