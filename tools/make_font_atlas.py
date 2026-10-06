#!/usr/bin/env python3
"""Renders the HUD font atlas: printable ASCII (32..126) plus a solid block (127) in a 16 x 6 grid of
12 x 26 pixel cells, white with the glyph coverage in the alpha channel.

    python3 tools/make_font_atlas.py RobotoMono.ttf crates/xp-app/assets/font/roboto-mono-atlas.png

Roboto Mono is licensed under the SIL Open Font License 1.1 (crates/xp-app/assets/font/LICENSE.txt).
Needs: pip install pillow
"""
import sys
from PIL import Image, ImageDraw, ImageFont

CELL_W, CELL_H, COLS, ROWS = 12, 26, 16, 6
font = ImageFont.truetype(sys.argv[1], 20)
atlas = Image.new('RGBA', (CELL_W * COLS, CELL_H * ROWS), (255, 255, 255, 0))
for code in range(32, 128):
    i = code - 32
    x, y = (i % COLS) * CELL_W, (i // COLS) * CELL_H
    cell = Image.new('L', (CELL_W, CELL_H), 0)
    if code == 127:
        ImageDraw.Draw(cell).rectangle([0, 0, CELL_W - 1, CELL_H - 1], fill=255)
    else:
        d = ImageDraw.Draw(cell)
        d.text((0, 1), chr(code), font=font, fill=255)
    atlas.paste(Image.merge('RGBA', (Image.new('L', cell.size, 255),) * 3 + (cell,)), (x, y))
atlas.save(sys.argv[2], optimize=True)
print('atlas', atlas.size)
