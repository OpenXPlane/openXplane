# Two-dimensional six-octave noise

Reference: X-Plane.exe SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Function `0x1411e1940` calls the existing bilinear basis `0x140984e50` six times.
`NoiseTable::fractal2` scales both coordinates by 1, 2, 4, 8, 16, 32,
rounding each double multiplication to float32. The double sum starts with
`0.5 * N2 + N1`, followed by weights 0.25, 0.125, 0.0625, 0.03125,
and rounds once to float32 at the end.

`tools/gen_noise2_vectors.py` executes the original instructions with no stubs,
checks completion and verifies the synthetic table remains unchanged. Its 1756
cases cover signed zeros, integer corners, neighbouring floats, wrapped table
indices, negative seeds and random coordinates. The Rust test reconstructs the
same 262144-entry table from the recorded integer hash recipe. No reference
assets or machine-code bytes are included in the fixture.

The API requires an explicit table. Its initialization in the live simulator
is not inferred from this synthetic table. Coordinates must remain finite and
below 2^32 at every octave; unsupported conversion edge cases are rejected.
The subsequent delta-wing modulation and whole-wing integration remain separate
verification tasks.
