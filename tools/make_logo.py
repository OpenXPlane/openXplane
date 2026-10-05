import math
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen

# --- NACA 2412 (the project's own reference airfoil) ---
def naca(m=0.03, p=0.4, t=0.17, n=48):
    up, lo = [], []
    for i in range(n + 1):
        b = (1 - math.cos(math.pi * i / n)) / 2
        yt = 5*t*(0.2969*math.sqrt(b) - 0.1260*b - 0.3516*b**2 + 0.2843*b**3 - 0.1036*b**4)
        yc = m/p**2*(2*p*b - b*b) if b < p else m/(1-p)**2*((1-2*p) + 2*p*b - b*b)
        dy = 2*m/p**2*(p-b) if b < p else 2*m/(1-p)**2*(p-b)
        th = math.atan(dy)
        up.append((b - yt*math.sin(th), yc + yt*math.cos(th)))
        lo.append((b + yt*math.sin(th), yc - yt*math.cos(th)))
    return up, lo

def airfoil_path(cx, cy, chord, aoa_deg):
    up, lo = naca()
    a = math.radians(aoa_deg)
    pts = up + lo[::-1]
    out = []
    for x, y in pts:
        x -= 0.5; y -= 0.0
        xr = x*math.cos(a) + y*math.sin(a)
        yr = -x*math.sin(a) + y*math.cos(a)
        out.append((cx + xr*chord, cy - yr*chord))
    return "M" + " L".join(f"{x:.2f} {y:.2f}" for x, y in out) + "Z"

def mark(size=64, bg_a="#0b4a74", bg_b="#04213a"):
    # Heavy italic X made of two slanted strokes, cyan gradient, with a offset shadow copy,
    # on a dark rounded square with a light cyan rim. Inside the X: airflow lines and our airfoil.
    strokes = "M10 14 H25 L54 50 H39 Z M39 14 H54 L25 50 H10 Z"
    af = airfoil_path(31, 33, 34, 8)
    return f'''<defs>
<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{bg_a}"/><stop offset="1" stop-color="{bg_b}"/></linearGradient>
<linearGradient id="x" gradientUnits="userSpaceOnUse" x1="14" y1="12" x2="50" y2="52"><stop offset="0" stop-color="#2fd2f5"/><stop offset="1" stop-color="#1c86d8"/></linearGradient>
<clipPath id="xc"><path transform="translate(32 32) skewX(-12) translate(-32 -32)" d="{strokes}"/></clipPath>
</defs>
<rect x="1.5" y="1.5" width="61" height="61" rx="15" fill="url(#bg)" stroke="#79e4ff" stroke-width="2.4"/>
<path transform="translate(30 34) skewX(-12) translate(-32 -32)" d="{strokes}" fill="#2a78bf" opacity=".85"/>
<path transform="translate(32 32) skewX(-12) translate(-32 -32)" d="{strokes}" fill="url(#x)"/>
<g clip-path="url(#xc)">
<g fill="none" stroke="#fff" stroke-linecap="round" opacity=".42" stroke-width="1.6">
<path d="M2 24 C 16 22 22 12 40 12 C 52 12 60 18 66 24"/>
<path d="M2 42 C 16 44 26 54 42 53 C 54 52 60 47 66 41"/>
</g>
<path d="{af}" fill="#fff" opacity=".5"/>
</g>'''

icon = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">{mark()}</svg>\n'
open('icon.svg','w').write(icon)

# --- wordmark: text outlined with Roboto (OFL) ---
base = TTFont('Roboto.ttf')
def font_at(w):
    return instantiateVariableFont(TTFont('Roboto.ttf'), {'wght': w, 'wdth': 100})
def text_path(font, text, x0, baseline, size, spacing=0):
    gs = font.getGlyphSet(); cmap = font.getBestCmap(); upm = font['head'].unitsPerEm
    s = size/upm; x = x0; d = []
    for ch in text:
        g = gs[cmap[ord(ch)]]
        pen = SVGPathPen(gs, ntos=lambda v: f"{v:.2f}")
        g.draw(TransformPen(pen, (s, 0, 0, -s, x, baseline)))
        d.append(pen.getCommands())
        x += g.width*s + spacing
    return " ".join(d), x

light, bold = font_at(400), font_at(800)
size = 40; base_y = 45
d1, x1 = text_path(light, "open", 72, base_y, size, -0.4)
d2, x2 = text_path(bold, "Xplane", x1+1, base_y, size, -0.4)
W = int(x2 + 8)
def wordmark(fg, accent):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} 64" width="{W}" height="64">
<g>{mark()}</g>
<path d="{d1}" fill="{fg}"/>
<path d="{d2}" fill="{accent}"/>
</svg>
'''
open('wordmark-dark.svg','w').write(wordmark('#1d2127', '#1e78d6'))   # for light backgrounds
open('wordmark-light.svg','w').write(wordmark('#ffffff', '#5aa5f0'))  # for dark backgrounds
print(W)
