# Actual main-menu panel Esc/Enter key route

Date: 2026-10-14. EXE SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Static creation path and concrete panel identity

The previously identified toggle_main_menu command requests kind 11 from
`0x14150e900`. Its getter/factory `0x14150b770` first checks the controller's
kind map; factory table `0x14150bd24` at index 11 resolves to `0x14150ba9d`.
That branch allocates 0x190 bytes and invokes constructor `0x140ed3d70`.
Kind 12 instead resolves to `0x14150bac0`, allocating 0x168 and invoking
`0x140ed25c0`. These are static branch identities, not a completed factory replay.

Kind-11 constructor installs final primary vtable `0x142604cc0` and creates
panel through `0x140ecb340` at call `0x140ed4a68`, storing its result in window
+0x188 at `0x140ed4a70`. The panel constructor's store at `0x140ecb472` installs
primary vtable `0x1426064f0`. This same construction region contains the actual
main_menu texture references recorded in `UI_RESOURCES.md`. Parent/selection
helpers called afterwards are not treated as evidence of a live focus assignment.

The panel primary table has key dispatch +0x50=`0x1412ee8a0` (recovered base
child/filter dispatcher), and own callback +0x38=`0x1412e1de0`. This is distinct
from the icon-button table whose own key callback always declines input. Static
constructor/table linkage identifies a concrete menu panel; actual runtime
construction, current focus chain and child graph remain unverified.

Panel callback subinterfaces at +0x170/+0x188/+0x1a0 have first entries
`0x140ed88b8`/`0x140ed88c4`/`0x140ed88d0`, adjusting this and tail-jumping to
`0x140ed9f00`. Those entries alone do not identify a command Begin callback or
show_menu ID 3. Numeric constants 3 in nearby layout/callback construction are
not used as evidence of command registration.

## Complete own key callback on this actual vtable

`0x1412e1de0..0x1412e1e8e` first requires flags bit 8. Without it, return is 0.
The character argument is unused; the full uint32 virtual key chooses behavior.

- Esc (27): getter slot +0x150 reads raw object byte +0x169. Nonzero requests
  action +0x178; zero requests action +0x170. Both paths return canonical 1.
- Return (13 or 0xbb): getter +0x138 reads raw byte +0x168. If nonzero, query
  +0x140 must permit action +0x168; otherwise return 0. If +0x168 byte is zero,
  use the same +0x169/action fallback as Esc. Successful paths return 1.
- All other virtual keys return 0, including 0xbc. The earlier acceptance-mask
  filter recognizes 0xbc as a Return-like input, but this own callback does not.

For **this concrete table**, +0x138=`0x1412e33e0` and
+0x150=`0x1412e33d0` are pure byte getters; +0x140=`0x140643a20` returns true;
+0x168/+0x178=`0x1405df230` are actual `ret 0` no-op bodies; and
+0x170=`0x140f00310` is a nontrivial action. Thus `ui_menu_key::dispatch`
specializes the full original own callback to that table. State retains opaque
`flag168`/`flag169` names; the flags' lifecycle/full semantic roles are unproven.
Only the +0x170 action is a service, named fallback_action without asserting
that it closes/resumes/opens any particular window.

This is not a replacement for the generic base callback on arbitrary widget
tables: other classes can override validation/actions. Child routing and mask
acceptance occur before this callback in `ui_input` and remain separate.

## Direct original-code evidence

`tools/gen_ui_menu_key_vectors.py` installs the original panel table and runs
complete own callback and, separately, full focused wrapper/base/filter/CRT with
explicit focus at the same panel and no child. Getters, constant query and no-op
bodies execute unchanged and are observed through nonmutating instruction hooks.
Only the +0x170 body is replaced by a logger applying declared flag changes.
No handler/getter/filter result is replaced.

1940 contexts (3880 complete native executions) compare exact return, final
flag bytes, ordered real getter/query/no-op/own-handler calls and fallback
requests, for both routes. They cover raw 0/1/255 and arbitrary flag bytes,
modifier/down/up/full flag words, full uint32 keys, Escape/Return/0xbb/0xbc,
char -1..255 inputs, varied acceptance masks and controlled fallback mutation.
No out-of-domain CRT input is supplied. A normal CI regression preserves
character-independence, distinct Return codes, no key-up action, and raw flag
semantics. Native comparison is opt-in/ignored in normal CI, and the numeric
corpus and original EXE remain local.

```sh
python tools/gen_ui_menu_key_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-menu-key.txt
OPENXPLANE_UI_MENU_KEY_VECTORS=/private/tmp/openxplane-ui-menu-key.txt cargo test --offline -p xp-app --test ui_menu_key_local -- --ignored
```

## Remaining behavior and visual acceptance

The concrete fallback `0x140f00310` starts by querying panel +0x28 interface
slot +0x10 with value 2 and testing returned float against zero (NaN follows
nonzero path). Later it queries engine state and can allocate another object.
Its complete effects are not ported or inferred from these partial observations.
That body, kind-11 creation/map/registry lifecycle, actual flags/focus/child graph,
show_menu ID-3 opening callback, Alt-only activation, original layout/fonts/GPU
rendering and paired screenshots/GIF/video remain outstanding. The active viewer
still exits on Esc and does not draw original menus. This key callback port adds
no pixel/motion or whole-game 1:1 proof.
