# Security

## Reporting a vulnerability

Please do not open a public issue for a security problem. Report it privately through GitHub:
**[Report a vulnerability](https://github.com/OpenXPlane/openXplane/security/advisories/new)** (the *Security*
tab of the repository). Only the maintainers see it.

Say what is affected, how to reproduce it and what an attacker could do with it. A small example (a file) helps.
The maintainers are volunteers: we aim to answer within a week and credit you in the advisory if you want.

## Supported versions

Only the latest nightly build.

## What counts

- A crafted content file (ACF, OBJ8, AFL, apt.dat) that crashes the reader, uses unbounded memory or reads
  outside the content folder.
- The Discord Rich Presence client sending anything other than the documented activity.
- The build and release workflows (secrets, artifact tampering).

The tools in `tools/` read the original executable and are meant for research on your own copy; they do not
write to it.
