# Direct menu action -> graph-backed close

The bridge reuses the complete recovered direct handler and overrides only its
close endpoint with `ui_menu_close_graph::dispatch`. All remaining direct effects
are forwarded unchanged. The caller supplies controller identity; mismatch
returns an error before modifying the graph. That validation is a port contract,
not a claim about native invalid-pointer behavior.

Ordinary composition tests verify re-entry guard suppression, actual graph alpha/
membership changes, map clearing, request order and mismatch error propagation.
The previously established 694 native direct-handler cases remain a separate
stage comparison. This bridge has **no combined original-code replay yet**;
separate stage evidence does not establish the full native menu route.

Next execute the complete original direct/close chain on one reference state,
then join the concrete fallback and own/focused Esc callback. Actual widget
construction, effect bodies and lifetime remain incomplete. Viewer rendering,
original Alt/Esc screenshots/GIF/video and whole-game parity remain unproven.
