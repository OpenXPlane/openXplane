# Menu key graph with owner/selected binding

`ui_menu_focus_graph` combines the complete menu key/fallback/direct/controller
chain with `ui_focus_binding` for actual owner/selected changes. Entry and widget
identities map to binding nodes; opaque binding targets map explicitly. Window
owner and widget bind endpoints execute the recovered helper instead of forwarding
the previous binding service. Other effects delegate unchanged.

Focused entry reads current global-focus identity from binding state. Null focus
returns 0; this executor supports the concrete panel and rejects other unsupported
widget classes. It captures that focus before invoking a selected child. Child
nonzero normalizes to 1 before acceptance filtering; declined child continues to
filter/own-key dispatch on the captured panel. Mask is supplied and stable in
this composition profile. Binding callback errors propagate as port errors;
invalid-state failure atomicity is not claimed.

## Verification achieved

Ordinary integration tests verify actual selected-link clearing and requested
owner stores during graph-backed close, callbacks changing focus, subsequent
null-focus key rejection, and child consumption before filter/window actions.
In the loss-of-focus test the controlled blur callback sets global focus null;
the binding helper itself does not write global focus. The next key receives no
calls after that change. Direct/fallback/key integration regressions also pass;
clippy passes.

The preceding 1045 original binding comparisons and 1456 original own/focused
key entries remain separate stage evidence. **No combined original-code replay
for this new focus composition exists yet.** The ordinary tests do not prove
original live focus, construction or virtual callback implementations.

## Next gate and remaining limits

Execute original key/close/reparent/owner binding on shared focus state, including
selected child callbacks and focus changes between keys. Virtual blur/+0x10,
child and remaining activation/preparation/query/document/dialog/lifetime effects
are controlled. Other focused widget classes, mask mutation through child effects,
object destruction and cross-handler global mutations need further work.

Real widget construction, show_menu ID3 opening/Alt activation, viewer integration,
original layout/fonts/GPU output and paired screenshots/GIF/video remain missing.
No visual evidence was produced. UI/graphics verified share remains zero and the
whole-game objective remains active.
