# Reference runtime and automatic capture investigation

Update2026-10-26: new original window captures now exist through Computer Use
after resetting its JavaScript kernel. See [the capture baseline](REFERENCE_WINDOW_BASELINE.md).
Native QA/picture export and a matched original/port scenario remain unresolved.

Date2026-10-26. Reference executable SHA256 matches [BASELINE.md](BASELINE.md).
The running log identifies itself as X-Plane12.4.3-r2-15ff1e4d, build124311,
compiled2026-06-24. This is a runtime label from the installed reference, not a
claim about other releases.

## Executed reference scenario

The installed `Output/replays/fps_test_kpdx.fps` is a1200 FPS replay with header
KVUO and the stock Cirrus Vision SF50. It is not the Cessna/KSEA port scenario.
Two completed runtime logs confirm these arguments:

```text
--fps_test=1 --require_fps=1 --fps_test_pics
--load_smo=Output/replays/fps_test_kpdx.fps
--no_sound --no_joysticks --window=1280x800 --disable_networking
```

The later run supplied `--fps_test_pics=5`. The first log reports152.4 seconds,
12574 frames,82.51fps; the second152.4 seconds,18389 frames,120.67fps. Both end
with clean thread shutdown and the normal X-Plane shutdown marker. These are
observed reference-run summaries, not paired performance comparisons. Loading,
installed plugins, caches and competing GPU use were not controlled. Logs show
installed plugins also ran; `--disable_networking` is not claimed to constrain
third-party plugin networking. Further reference runs should control plugins.

On this Windows installation, directly invoking a GUI executable from the shell
did not reliably wait for the actual game. One observed launcher exited53 while
a later real process and updated log established execution. Process command-line
inspection confirmed the exact target and arguments. An absent launcher is not
proof that the game is stopped. A future runner must track the actual target
process and log identity and must not relaunch on an observation timeout.

No new reference PNG was found in Output after either completed run. The absence
of files there does not prove that no other path or format was used. No screenshot
or visual-parity claim follows from successful FPS completion.

## REA instruction evidence for picture option

REA6.7.0 `inspect-with-objdump`, through the disclosed range adapter
`tools/rea_bounded_objdump.rs` selector `fps-pics`, produced completed Evidence
`ev_69c08237c9e80fe9b3b423a46be41d2e5270314425a6991e93d1dd9a4b8c8666`.
The half-open range140e6d3c1..140e6d44b contains138 bytes; every emitted byte was
checked against the original mapped PE. Exit0, untruncated output, unchanged
input and before/after reference hashes were checked. Original instructions and
full Evidence stay in ignored local research.

The block queries the literal `--fps_test_pics`, obtains its string value,
selects inline or heap data using capacity15, calls numeric conversion142309af8,
converts its double result to float and stores receiver+1674. It then cleans up
the temporary using the native heap/alignment rules. Thus the option has a
numeric value; its name alone does not establish a Boolean screenshot switch.
The unit, scheduling, capture destination, interaction with `--require_fps` and
downstream use of receiver+1674 remain unresolved. The two runs do not prove
the semantics of an empty value versus5.

## Port observations and next steps

The current port produced new1280x800 Cessna/KSEA frames at model times0,20,40s.
At20s its approximate model reports69.3kt,433.3ft,pitch5.3deg; at40s89.4kt,
556.8ft,pitch7.5deg. These are port-only traces, not matched native trajectories.
The offscreen exporter now omits unmeasured timing line0 instead of injecting
fixed reference-photograph values. Other requested rows use model state.

Next recover downstream picture scheduling/destination and run the same
aircraft/location/camera/time in both engines. Restore Computer Use (both
runtimes previously failed before initialization with a kernel-assets path
error) for real menu/Alt/Esc and motion captures. Until then, original FPS logs,
port frames and static instruction evidence remain separate scopes.
