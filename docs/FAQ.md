# FAQ

**Is this X-Plane?** No. openXplane is an independent project that reads X-Plane 12 content and aims to behave
like it. It is not made by, affiliated with or endorsed by Laminar Research.

**Do I need X-Plane?** Yes. The project contains none of its content; you point it at your own installation.

**Can I fly?** The Cessna 172 flies with an approximate flight model that is partly the original's (the airfoil
and element aerodynamics are verified against it) and partly our own. The engine, gear and flight loop are not
ported yet. See [COMPATIBILITY.md](COMPATIBILITY.md) for the estimate.

**How is "identical to the original" checked?** Functions of the original executable run in an emulator on
synthetic inputs; the Rust port must return the same bits ([research/VERIFICATION.md](../research/VERIFICATION.md)).

**Why Rust and wgpu?** Memory safety, one code base for Windows, macOS and Linux, and a modern graphics layer
(Vulkan, Metal, DirectX 12).

**What licence is it under?** GPL-3.0-or-later, see [LICENSE](../LICENSE) and [NOTICE](../NOTICE). The documents under `research/` describe observed
behaviour in our own words; no original code or assets are in the repository.

**Does it work with scenery or aircraft from the store?** Only what the readers support: ACF, OBJ8, AFL and
`apt.dat` today. DSF terrain is not read yet.

**Discord shows a status for me.** That is the optional Rich Presence; turn it off by setting
`OPENXPLANE_DISCORD_APP_ID` to an empty value ([DISCORD.md](DISCORD.md)).

**Where do I report a problem?** [GitHub issues](https://github.com/OpenXPlane/openXplane/issues), with the
command you ran and its output.
