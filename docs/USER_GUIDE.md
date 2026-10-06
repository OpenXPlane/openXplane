# User guide

openXplane is an independent engine that reads the content of an X-Plane 12 installation you already own. It
is early software: it can show aircraft and airports and fly the Cessna 172 with an approximate flight model.
[Compatibility](COMPATIBILITY.md) says how much of X-Plane it reproduces.

## What you need

- A copy of **X-Plane 12**. openXplane contains no aircraft, scenery, airfoils or sounds of its own.
- Windows, macOS or Linux with a GPU that supports Vulkan, Metal or DirectX 12.

## Install

1. Download the archive for your system from the
   [nightly release](https://github.com/OpenXPlane/openXplane/releases/tag/nightly) and unpack it.
2. Keep your X-Plane folder where it is; openXplane reads it in place and never changes it.
3. Run the program from a terminal, giving the X-Plane folder (the one with `Aircraft`, `Resources`,
   `Custom Scenery`) or, for the flat provided layout, its `Xplane12` folder.

## First flights

```sh
openxplane view "<X-Plane>/Aircraft/Laminar Research/Cessna 172 SP/Cessna_172SP.acf"   # look at an aircraft
openxplane airport-view <X-Plane> KSEA "<path to the .acf>"                               # aircraft on a runway
openxplane fly <X-Plane> KSEA "<path to the .acf>"                                        # fly it
```

`fly` uses the original's default keys: `F1`/`F2`/`F3` throttle, `1`/`2` flaps, `B` brakes, `[` `]` trim,
`P` pause. The arrow keys and `Z`/`X` are a keyboard stick that the original does not have (`Tab` gives them
their original meaning back); `Delete` resets, `Esc` quits. The complete list is in the
[keyboard reference](KEYBOARD.md).

## Data output

X-Plane can print rows of numbers over the scene (its Data Output settings). openXplane draws the same rows,
labelled as the original labels them, for the lines it can fill:

```sh
OPENXPLANE_DATA_OUTPUT=0,3,4,17,18,21 openxplane fly <X-Plane> KSEA "<path to the .acf>"
```

The numbers are indexes of the original's table (0 frame rates, 3 speeds, 4 Mach and vertical speed, 8 stick,
13 trims and flaps, 17 attitude, 18 angle of attack and flight path, 20 altitude, 21 position and velocity,
25 throttle). The values come from the approximate flight model.

## Discord

While a window is open, openXplane shows what you are doing as your Discord status, through the local Discord
app only. Set `OPENXPLANE_DISCORD_APP_ID` to an empty value to turn it off ([details](DISCORD.md)).

## When something is wrong

- *"missing ACF property" or file errors*: point the program at the folder that contains the aircraft; some
  third-party aircraft use features that are not read yet. `openxplane inspect <X-Plane>` lists what is missing.
- *No window or a black window*: update your graphics driver; run with `--smoke` to test the renderer.
- Anything else: open an [issue](https://github.com/OpenXPlane/openXplane/issues) with the command and its output.
