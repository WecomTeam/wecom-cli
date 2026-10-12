#!/usr/bin/env python3
"""Assemble slide pages from a fixed shell.

  init:     python3 build_page.py init --set signal   [--map colors.json]              (copy shells + template pages + preset into cwd, recoloured)
  template: python3 build_page.py template <user.pptx>  then  template --docid <docid>          (shells built FROM a user's PPT template)
  buildall: python3 build_page.py buildall                                             (build every NN_body.jsx, then contrast + images, one command)

Older single-step commands: so the header, footer, rules and page padding never drift.

  build: python3 build_page.py build --shell 00_shell.jsx --body 05_body.jsx --vars 05_vars.json [--manifest images.json] --out 05_market.jsx
  check: python3 build_page.py check --shell 00_shell.jsx 03_x.jsx 04_y.jsx ...
  recolor: python3 build_page.py recolor --map colors.json --out-dir local/ shell/content.jsx slides/*.slide   (swap a set's palette)
  contrast: python3 build_page.py contrast 01_cover.jsx 02_x.jsx ...   (text vs. the surface it sits on)
  fit: python3 build_page.py fit 01_cover.jsx 02_x.jsx ...   (does the text fit its box; absolute text landing on other text; Slide must be 1280x720)
  imgfit: python3 build_page.py imgfit --slot 576x434 --actual 1024x1024 --src <url> [--bg #DCEBFD]   (prints the Image JSX)
  images: python3 build_page.py images --manifest images.json 04_x.jsx ...   (written size vs. real aspect ratio)

The shell is a copy of templates/<slug>/shell/content.jsx. Placeholders look like 〔标题〕; 〔B区内容〕 is
where the page body goes. Values are inserted as-is, so they may contain <br /> or <span> markup. Pass "" to leave a
placeholder empty on purpose. `check` verifies that every page is the shell with only the placeholders filled in.
"""
import argparse, json, re, sys

BODY = '〔B区内容〕'
PH = re.compile(r'〔[^〔〕]*〕')

def build(a):
    shell = open(a.shell, encoding='utf-8').read()
    body = open(a.body, encoding='utf-8').read().strip('\n')
    values = json.load(open(a.vars, encoding='utf-8')) if a.vars else {}
    values = {(k[1:-1] if k.startswith('〔') and k.endswith('〕') else k): v for k, v in values.items()}   # "〔标题〕" keys too
    if BODY not in shell and body.strip(): sys.exit(f'shell has no {BODY} marker: {a.shell} -- this page type takes no body: leave its body file empty')
    missing = []
    def fill(m):
        key = m.group(0)[1:-1]
        if m.group(0) == BODY: return m.group(0)
        if key in values: return str(values[key])
        missing.append(key); return m.group(0)
    page = PH.sub(fill, shell)
    if missing:
        allkeys = [k[1:-1] for k in dict.fromkeys(PH.findall(shell)) if k != BODY]
        ready = json.dumps({k: '' for k in allkeys}, ensure_ascii=False, indent=1)
        shellname = a.shell.replace('\\', '/').rsplit('/', 1)[-1]
        sys.exit('missing values for: ' + ', '.join(sorted(set(missing))) +
                 f'\nthese are placeholders that the shell {shellname} needs -- its header, page number, footer'
                 '\nand lead-in question live in the shell, not in the body, so every page needs its own vars file.'
                 f'\nadd them to {a.vars or "--vars"} (use "" to leave one empty). Full set of keys for this shell:\n'
                 + ready)
    if BODY in page:
        indent = re.search(r'^([ \t]*)' + re.escape(BODY), page, re.M).group(1)
        page = page.replace(indent + BODY, '\n'.join(indent + ln if ln.strip() else ln for ln in body.split('\n')), 1)
    if 'tpl_assets/' in page:                          # art of a user template: urls come from `template --docid`
        import os
        up = os.path.join(os.path.dirname(os.path.abspath(a.shell)), 'tpl_urls.json')
        for rel, url in (json.load(open(up, encoding='utf-8')).items() if os.path.exists(up) else []):
            page = page.replace(f'"{rel}"', f'"{url}"')
        if 'tpl_assets/' in page:
            sys.exit('template art not uploaded yet (' + ', '.join(sorted(set(re.findall(r'tpl_assets/[\w.\-]+', page)))) +
                     ') -> run `template --docid <docid>` once after the document exists, then rerun buildall')
    if '<Image' in page:
        if not a.manifest: sys.exit('page contains <Image>: pass --manifest images.json (real sizes recorded in Step6) so the picture cannot be squashed')
        issues = _image_issues(page, json.load(open(a.manifest, encoding='utf-8')))
        if issues: sys.exit('image would be squashed, page not written:\n  ' + '\n  '.join(issues) + '\nrun imgfit with the real size and paste its output into the body')
    open(a.out, 'w', encoding='utf-8').write(page if page.endswith('\n') else page + '\n')
    print(f'ok {a.out}')

def shell_regex(shell):
    parts, pos = [], 0
    for m in PH.finditer(shell):
        parts.append(re.escape(shell[pos:m.start()]))
        parts.append(r'[\s\S]*?' if m.group(0) == BODY else r'(?:(?!</?Box|</?Slide|<Image|<Chart|<Table|<svg)[\s\S])*?')
        pos = m.end()
    parts.append(re.escape(shell[pos:]))
    rx = ''.join(parts)
    return re.compile(r'\s*' + re.sub(r'(\\\s|\\\n|\\ )+', r'\\s*', rx) + r'\s*')

def check(a):
    shell = open(a.shell, encoding='utf-8').read().strip()
    rx, bad = shell_regex(shell), 0
    for p in a.pages:
        src = open(p, encoding='utf-8').read()
        if rx.fullmatch(src): print(f'OK    {p}'); continue
        bad += 1; print(f'DRIFT {p}')
        fixed = [ln.strip() for ln in PH.sub('\0', shell).split('\n') if ln.strip() and '\0' not in ln]
        gone = [ln for ln in fixed if ln not in src]
        for ln in gone[:6]: print('      shell line not found verbatim: ' + ln[:110])
        if not gone: print('      every fixed line is present, but order or placeholder lines differ from the shell')
    sys.exit(1 if bad else 0)

HEX = re.compile(r"#([0-9A-Fa-f]{6})\b")
def _lum(h):
    c = [int(h[i:i+2], 16) / 255 for i in (0, 2, 4)]
    c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
def _ratio(a, b):
    la, lb = sorted((_lum(a), _lum(b)), reverse=True); return (la + 0.05) / (lb + 0.05)

_TAG = re.compile(r"<(/?)(Slide|Box|Text|span)\b((?:[^<>]|=>)*?)(/?)>", re.S)

def _contrast_hits(src):
    """Every text/surface pair in one page source that is under its threshold, as
    (line, text hex, surface hex, font size, ratio, needed, 'FAIL' | 'warn')."""
    stack, size, hits = ['FFFFFF'], [16], []
    for m in _TAG.finditer(src):
        close, name, attrs, selfc = m.group(1), m.group(2), m.group(3), m.group(4)
        if close:
            if name in ('Slide', 'Box') and len(stack) > 1: stack.pop()
            if name in ('Text', 'span') and len(size) > 1: size.pop()
            continue
        bg = re.search(r"background(?:Color)?:\s*'#([0-9A-Fa-f]{6})'", attrs)
        fs = re.search(r"fontSize:\s*(\d+)", attrs); col = re.search(r"(?<![A-Za-z])color:\s*'#([0-9A-Fa-f]{6})'", attrs)
        if name in ('Slide', 'Box'):
            grad = re.search(r"gradient|backgroundImage|url\(", attrs)
            if not selfc: stack.append(None if grad else (bg.group(1) if bg else stack[-1]))
            continue
        cur = int(fs.group(1)) if fs else size[-1]
        if not selfc: size.append(cur)
        if col and 'backgroundClip' not in attrs and stack[-1] and cur < 150:
            r = _ratio(col.group(1), stack[-1]); need = 3.0 if cur >= 24 else 4.5
            if r < need: hits.append((src.count('\n', 0, m.start()) + 1, col.group(1), stack[-1], cur, r, need, 'FAIL' if r < 3.0 else 'warn'))
    return hits

_DIGIT_HEX = re.compile(r"#(\d{6})\b")

def _hexsafe(h):
    """The nearest colour whose hex code contains a letter. The WeCom exporter turns an all-digit value such as
    #184785 or #244972 into pure red (FF0000) in the .pptx, so no colour may be written with six decimal digits."""
    h = h.lstrip('#').upper()
    if not h.isdigit(): return h
    for d in range(1, 16):
        for i in range(3):
            v = int(h[2 * i:2 * i + 2], 16)
            for cand in (v + d, v - d):
                if 0 <= cand <= 255 and not f'{cand:02X}'.isdigit(): return h[:2 * i] + f'{cand:02X}' + h[2 * i + 2:]
    return h

def _digit_hits(src):
    return [(src.count('\n', 0, m.start()) + 1, m.group(1)) for m in _DIGIT_HEX.finditer(src)]

_SVG_OPEN = re.compile(r"<svg\b([^>]*)>")

def _svg_hits(src):
    """SVGs whose PNG fallback the WeCom exporter would flatten onto white. The exporter stores every <svg> twice,
    as the vector plus a rasterised PNG, and the PNG loses all transparency; Keynote, Quick Look and the preview
    viewers read that PNG. So the first drawn child must be an opaque <rect> covering the whole viewBox in the
    colour of the surface under it, the svg's own style may not carry opacity, and no paint may be rgba()."""
    hits = []
    for m in _SVG_OPEN.finditer(src):
        ln = src.count('\n', 0, m.start()) + 1; attrs = m.group(1)
        end = src.find('</svg>', m.end()); body = src[m.end():end if end > 0 else len(src)]
        vb = re.search(r"viewBox=['\"]0 0 ([\d.]+) ([\d.]+)['\"]", attrs)
        w = vb.group(1) if vb else (re.search(r"\bwidth=\{?['\"]?([\d.]+)", attrs) or [None, None])[1]
        h = vb.group(2) if vb else (re.search(r"\bheight=\{?['\"]?([\d.]+)", attrs) or [None, None])[1]
        if re.search(r"\bopacity:\s*[\d.]+", attrs):
            hits.append((ln, "svg style carries opacity -> drop it and premix the colours with the surface (the exporter's PNG fallback ignores it)"))
        if 'rgba(' in body:
            hits.append((ln, "rgba() inside <svg> is not a valid SVG paint -> write a hex colour plus fillOpacity / strokeOpacity "
                             "(does not blank the graphic, but the PNG fallback may drop the colour)"))
        for am in re.finditer(r'&', body):
            aln = src.count('\n', 0, m.end() + am.start()) + 1
            hits.append((aln, "'&' inside <svg> -> on export it becomes a bare '&', the svg turns into malformed XML and the WHOLE "
                              "graphic renders blank. Rewrite as 与 / and / ·. &amp;, &#38; and '\\u0026' all export to the same "
                              "bare '&' and do NOT help; '&' outside <svg> (e.g. in <Text>) is fine."))
        if w:
            for tm in re.finditer(r"<text\b([^>]*)>([^<{}]+)</text>", body):
                ta, tb = tm.group(1), tm.group(2).strip()
                mx = re.search(r"\bx=['\"{]?(-?[\d.]+)", ta)
                if not tb or not mx:
                    continue
                mfs = re.search(r"font-?[Ss]ize=['\"{]?([\d.]+)", ta)
                x, fs = float(mx.group(1)), float(mfs.group(1)) if mfs else 16.0
                # CJK full width, half width 0.55em
                tw = sum(1.0 if ord(c) > 0x2E80 else 0.55 for c in tb) * fs
                anchor = (re.search(r"text-?[Aa]nchor=['\"](\w+)", ta) or [None, 'start'])[1]
                x0, x1 = (x - tw / 2, x + tw / 2) if anchor == 'middle' else \
                         ((x - tw, x) if anchor == 'end' else (x, x + tw))
                tln = src.count('\n', 0, m.end() + tm.start()) + 1
                if x1 > float(w) + 0.5:
                    hits.append((tln, f"svg text runs to {x1:.0f}, past the viewBox width {w} -> it gets hard-clipped. "
                                      f"Move it left, shorten it, or widen the viewBox: {tb[:34]!r}"))
                elif x0 < -0.5:
                    hits.append((tln, f"svg text starts at {x0:.0f}, left of the viewBox -> it gets hard-clipped: {tb[:34]!r}"))
        inner = re.sub(r"<defs>.*?</defs>", '', body, flags=re.S)
        first = re.search(r"<([A-Za-z]+)\b([^>]*)>", inner); ok = False
        if first and first.group(1) == 'rect':
            ra = first.group(2)
            rw = re.search(r"\bwidth=['\"{]?([\d.]+%?)", ra); rh = re.search(r"\bheight=['\"{]?([\d.]+%?)", ra)
            fill = re.search(r"\bfill=['\"]#[0-9A-Fa-f]{6}['\"]", ra)
            cov = lambda v, need: v is not None and (v.group(1) == '100%' or need is None or float(v.group(1)) >= float(need))
            ok = bool(fill) and cov(rw, w) and cov(rh, h) and 'opacity' not in ra.lower()
        if not ok:
            hits.append((ln, f"svg has no opaque backing rect -> its first drawn child must be <rect x='0' y='0' width='{w or 'W'}' height='{h or 'H'}' fill='#<colour of the surface under it>' /> (the exporter's PNG fallback turns transparency into white)"))
    return hits

_ABS_TAG = re.compile(r"<(Box|Text|Image|svg)\b([^>]*?)/?>", re.S)
_ABS_STYLE = re.compile(r"style=\{\{(.*?)\}\}", re.S)


def _sprop(style, name):
    m = (re.search(rf"\b{name}\s*:\s*'([^']*)'", style) or
         re.search(rf"\b{name}\s*:\s*\"([^\"]*)\"", style) or
         re.search(rf"\b{name}\s*:\s*([\d.-]+)", style))
    return m.group(1).strip() if m else None


def _spx(v, base):
    """'700px' / '50%' / 60 -> pixels (base is used for percentages)."""
    if v is None:
        return None
    s = str(v).strip()
    m = re.match(r"^(-?[\d.]+)\s*%$", s)
    if m:
        return float(m.group(1)) / 100.0 * base
    m = re.match(r"^(-?[\d.]+)(?:px)?$", s)
    return float(m.group(1)) if m else None


def _abs_hits(src, W=1280.0, H=720.0):
    """Absolutely positioned elements that stick out of the canvas. The renderer rejects the whole page for
    these (content overflow) instead of just clipping them, and a flow-layout height check will not catch
    them because they sit outside the flow. Mirrors the renderer's own maths:
        top  = pct(top)*H  - pct(translateY)*height
        left = pct(left)*W - pct(translateX)*width"""
    hits = []
    for m in _ABS_TAG.finditer(src):
        tag, attrs = m.group(1), m.group(2)
        ms = _ABS_STYLE.search(attrs)
        if not ms or _sprop(ms.group(1), 'position') != 'absolute':
            continue
        st = ms.group(1)
        ln = src.count('\n', 0, m.start()) + 1
        w, h = _spx(_sprop(st, 'width'), W), _spx(_sprop(st, 'height'), H)
        dx = dy = 0.0
        tr = _sprop(st, 'transform')
        if tr:
            tm = re.search(r"translate\(\s*([^,)]+)(?:,\s*([^)]+))?\)", tr)
            if tm:
                dx = _spx(tm.group(1), w or 0) or 0.0
                dy = (_spx(tm.group(2), h or 0) or 0.0) if tm.group(2) else 0.0
            for fn, idx in (("translateX", 'x'), ("translateY", 'y')):
                fm = re.search(rf"{fn}\(\s*([^)]+)\)", tr)
                if fm:
                    val = _spx(fm.group(1), (w or 0) if idx == 'x' else (h or 0)) or 0.0
                    if idx == 'x':
                        dx = val
                    else:
                        dy = val
        top, left = _spx(_sprop(st, 'top'), H), _spx(_sprop(st, 'left'), W)
        bot, rgt = _spx(_sprop(st, 'bottom'), H), _spx(_sprop(st, 'right'), W)

        y0 = y1 = x0 = x1 = None
        if top is not None:
            y0 = top + dy
            y1 = y0 + h if h is not None else None
        elif bot is not None and h is not None:
            y1 = H - bot + dy
            y0 = y1 - h
        if left is not None:
            x0 = left + dx
            x1 = x0 + w if w is not None else None
        elif rgt is not None and w is not None:
            x1 = W - rgt + dx
            x0 = x1 - w

        for val, lim, side in ((y0, 0, 'top'), (x0, 0, 'left')):
            if val is not None and val < -0.5:
                hits.append((ln, f"<{tag}> sticks out past the {side} of the canvas by {-val:.0f}px "
                                 f"({side} edge at {val:.0f}) -> the page is rejected as overflowing. "
                                 f"Pull it back inside (non-negative offsets, translate at most -50%)"))
        if y1 is not None and y1 > H + 0.5:
            hits.append((ln, f"<{tag}> sticks out past the bottom of the canvas by {y1-H:.0f}px "
                             f"(bottom edge at {y1:.0f}) -> the page is rejected as overflowing"))
        if x1 is not None and x1 > W + 0.5:
            hits.append((ln, f"<{tag}> sticks out past the right of the canvas by {x1-W:.0f}px "
                             f"(right edge at {x1:.0f}) -> the page is rejected as overflowing"))
        for k in ('top', 'left', 'bottom', 'right'):
            raw = _sprop(st, k)
            if raw and str(raw).startswith('-'):
                hits.append((ln, f"<{tag}> {k}:{raw} is a negative offset -> not allowed, "
                                 f"position with positive values instead (e.g. top:40, right:40)"))
    return hits


def contrast(a):
    """Flag text whose colour is too close to the surface it sits on (nearest ancestor with a solid background).
    Contrast under 3.0 fails; small text between 3.0 and 4.5 is only counted as a warning. Gradient surfaces and
    decorative numerals of 150px and up are skipped. Also flags what the WeCom exporter breaks (six-digit colour
    codes, SVGs without an opaque backing rect, '&' inside an svg, svg text past the viewBox) and absolutely
    positioned elements that stick out of the 1280x720 canvas."""
    bad = 0
    for p in a.pages:
        src = open(p, encoding='utf-8').read()
        hits = _contrast_hits(src); digits = _digit_hits(src); svgs = _svg_hits(src); abso = _abs_hits(src)
        fails = [h for h in hits if h[6] == 'FAIL']
        print(('LOW   ' if fails or digits or svgs or abso else 'OK    ') + p + (f'  ({len(hits) - len(fails)} small-text warnings under 4.5)' if len(hits) > len(fails) else ''))
        for ln, c, b, fsz, r, need, lvl in fails[:20]: print(f'      line {ln}: text #{c} on #{b}, {fsz}px, contrast {r:.2f} < 3.0 -> unreadable, change the text or surface colour')
        for ln, c in digits[:20]: print(f'      line {ln}: #{c} is six decimal digits; the WeCom exporter turns such values into pure red (FF0000) -> write #{_hexsafe(c)}')
        for ln, msg in svgs[:20]: print(f'      line {ln}: {msg}')
        for ln, msg in abso[:20]: print(f'      line {ln}: {msg}')
        bad += bool(fails or digits or svgs or abso)
    sys.exit(1 if bad else 0)

def _dims_from_bytes(b):
    """Return (w, h) for PNG / JPEG / GIF / WebP headers, or None."""
    import struct
    if b[:8] == b'\x89PNG\r\n\x1a\n' and len(b) >= 24: return struct.unpack('>II', b[16:24])
    if b[:6] in (b'GIF87a', b'GIF89a') and len(b) >= 10: return struct.unpack('<HH', b[6:10])
    if b[:4] == b'RIFF' and b[8:12] == b'WEBP' and len(b) >= 30:
        if b[12:16] == b'VP8 ': return struct.unpack('<HH', b[26:30])[0] & 0x3fff, struct.unpack('<HH', b[26:30])[1] & 0x3fff
        if b[12:16] == b'VP8L': bits = int.from_bytes(b[21:25], 'little'); return (bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1
        if b[12:16] == b'VP8X': return int.from_bytes(b[24:27], 'little') + 1, int.from_bytes(b[27:30], 'little') + 1
    if b[:2] == b'\xff\xd8':
        i = 2
        while i + 9 < len(b):
            if b[i] != 0xff: i += 1; continue
            marker = b[i + 1]; seg = int.from_bytes(b[i + 2:i + 4], 'big')
            if marker in (0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf):
                return int.from_bytes(b[i + 7:i + 9], 'big'), int.from_bytes(b[i + 5:i + 7], 'big')
            i += 2 + seg
    return None

def _fetch_dims(url, timeout=8):
    import urllib.request
    try:
        req = urllib.request.Request(url, headers={'Range': 'bytes=0-65535', 'User-Agent': 'build_page'})
        with urllib.request.urlopen(req, timeout=timeout) as r: return _dims_from_bytes(r.read(65536))
    except Exception: return None

def imgfit(a):
    """Print the JSX for an image slot so the picture keeps its real aspect ratio. Never stretches, never relies on objectFit
    (the WeCom compiler crops as if every picture were square, so a non-square picture filled into a slot gets squashed)."""
    sw, sh = map(float, a.slot.lower().split('x')); aw, ah = map(float, a.actual.lower().split('x'))
    slot_r, img_r = sw / sh, aw / ah; dev = abs(img_r - slot_r) / slot_r
    if dev <= 0.03:
        w, h = int(sw), int(sh); note = f'ratio matches slot ({dev*100:.1f}% off), fill the slot'
    elif img_r > slot_r:
        w, h = int(sw), round(sw / img_r); note = f'image wider than slot ({dev*100:.0f}% off): full width, {h}px tall, letterboxed'
    else:
        h, w = int(sh), round(sh * img_r); note = f'image taller than slot ({dev*100:.0f}% off): full height, {w}px wide, pillarboxed'
    bg = f", background: '{a.bg}'" if a.bg else ''
    print(f"<Box style={{{{ width: {int(sw)}, height: {int(sh)}, justifyContent: 'center', alignItems: 'center'{bg} }}}}>")
    print(f"    <Image src=\"{a.src}\" style={{{{ width: {w}, height: {h} }}}} />")
    print("</Box>")
    print(f"// {note}" if False else "", end='')
    sys.stderr.write(note + ('\n  consider regenerating at the slot ratio instead of letterboxing' if dev > 0.03 else '') + '\n')

def _image_issues(src, manifest, fetch=True):
    tag = re.compile(r'<Image\b[^>]*?src=["\']([^"\']+)["\'][^>]*?>', re.S); issues = []
    for m in tag.finditer(src):
        url = m.group(1); attrs = m.group(0)
        w = re.search(r'width:\s*(\d+(?:\.\d+)?)', attrs); h = re.search(r'height:\s*(\d+(?:\.\d+)?)', attrs)
        if not (w and h): issues.append(f'{url[:60]}: width/height not both numeric'); continue
        if url.startswith('tpl_assets/'): issues.append(f'{url}: template art not uploaded yet -> run `template --docid <docid>` once'); continue
        act = manifest.get(url) or (_fetch_dims(url) if fetch else None)
        if isinstance(act, dict) and act.get('stretch'): continue    # a template's own page background, stretched to 16:9 on purpose
        if not act: issues.append(f'{url[:60]}: real size unknown (not in manifest, not fetchable) -> record it in images.json'); continue
        aw, ah = act if isinstance(act, (list, tuple)) else (act['width'], act['height'])
        dev = abs((aw / ah) - (float(w.group(1)) / float(h.group(1)))) / (aw / ah)
        if dev > 0.03: issues.append(f'{url[:60]}: real {aw}x{ah} vs written {w.group(1)}x{h.group(1)}, ratio off by {dev*100:.0f}% -> squashed; use imgfit')
    return issues

def images(a):
    """Check every <Image> in the pages: its written width/height must match the picture's real aspect ratio (within 3%)."""
    manifest = json.load(open(a.manifest, encoding='utf-8')) if a.manifest else {}; bad = 0
    for p in a.pages:
        issues = _image_issues(open(p, encoding='utf-8').read(), manifest)
        print(('BAD   ' if issues else 'OK    ') + p); [print('      ' + i) for i in issues]; bad += bool(issues)
    sys.exit(1 if bad else 0)

_TEMPLATE_DIRS = (('..', 'assets', 'templates'), ('templates',), ('..', 'references', 'templates'))

def _templates_root():
    """Locate the template pool. Runtimes ship it under assets/ (sandbox) or references/ (skill tree)."""
    import os
    here = os.path.dirname(os.path.abspath(__file__))
    for rel in _TEMPLATE_DIRS:
        cand = os.path.join(here, *rel)
        if os.path.isdir(cand): return cand
    sys.exit('template pool not found next to build_page.py; looked in: '
             + ', '.join('/'.join(r) for r in _TEMPLATE_DIRS))

def _set_root(name):
    import os
    root = os.path.join(_templates_root(), name)
    if not os.path.isdir(root): sys.exit(f'no such set: {name} (use the flat slug, e.g. signal)')
    return root

def _set_files(root):
    import os, glob
    return sorted(glob.glob(os.path.join(root, 'shell', '*.jsx'))) + sorted(glob.glob(os.path.join(root, 'slides', '*.slide')))

def _rgb_to_hsl(h):
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    mx, mn = max(r, g, b), min(r, g, b); l = (mx + mn) / 2
    if mx == mn: return 0.0, 0.0, l
    d = mx - mn; s = d / (2 - mx - mn) if l > 0.5 else d / (mx + mn)
    if mx == r: hu = ((g - b) / d) % 6
    elif mx == g: hu = (b - r) / d + 2
    else: hu = (r - g) / d + 4
    return hu * 60, s, l

def _hsl_to_rgb(hu, s, l):
    hu = hu % 360; c = (1 - abs(2 * l - 1)) * s; x = c * (1 - abs((hu / 60) % 2 - 1)); m = l - c / 2
    r, g, b = [(c, x, 0), (x, c, 0), (0, c, x), (0, x, c), (x, 0, c), (c, 0, x)][int(hu // 60) % 6]
    return '%02X%02X%02X' % tuple(round((v + m) * 255) for v in (r, g, b))

def _at_luminance(hu, s, target):
    """The hue/saturation asked for, with lightness tuned until the colour has the given relative luminance.
    Matching luminance (not HSL lightness) is what keeps every contrast ratio of the original palette intact:
    HSL lightness lies, a yellow and a blue at the same lightness are nowhere near the same brightness."""
    lo, hi = 0.0, 1.0
    for _ in range(24):
        mid = (lo + hi) / 2
        if _lum(_hsl_to_rgb(hu, s, mid)) < target: lo = mid
        else: hi = mid
    return _hsl_to_rgb(hu, s, (lo + hi) / 2)

def palette(a):
    """Derive a whole palette for a set from one hue, keeping every colour's luminance (so the contrast the set was
    built with survives exactly) and the hue distances between colours (so multi-colour sets stay distinguishable).
    Writes colors.json for `init --map` and checks every text/surface pair before doing so."""
    import os, json as _json, collections
    root = _set_root(a.set); files = _set_files(root)
    srcs = {f: open(f, encoding='utf-8').read() for f in files}
    counts = collections.Counter(h.upper() for s in srcs.values() for h in HEX.findall(s))
    if not counts: sys.exit(f'no colours found in {a.set}')
    hsl = {h: _rgb_to_hsl(h) for h in counts}
    chroma = [h for h in counts if hsl[h][1] >= 0.12]
    base_hue = hsl[max(chroma, key=lambda h: counts[h])][0] if chroma else 0.0
    gap = abs((a.hue - base_hue + 180) % 360 - 180)     # how far the asked-for hue is from the set's own
    if chroma and not a.invert and not a.force and gap < a.min_gap:
        print(f"{a.set} is already a {base_hue:.0f} hue set; --hue {a.hue:.0f} is only {gap:.0f} deg away "
              f"(gate: {a.min_gap:.0f}).\nA palette this close to the default is the default with the edges filed "
              "off: it clears every contrast check and\nstill looks like the set's own colours, so it is a recolour "
              "in name only.")
        print("\nUSE THE SET'S DEFAULT PALETTE. Run init WITHOUT --map, and record in DESIGN.md that the colour "
              "intent\nmatches this set's default (state both hues and the gap). That is a legitimate outcome, not "
              "a failure -- do\nNOT go hunting for some other hue just to get past this gate, and do NOT hand-write "
              "a map.")
        print(f"\nOnly if the deck genuinely needs a different colour: pick a hue at least {a.min_gap:.0f} deg from "
              f"{base_hue:.0f} (i.e. outside\n{(base_hue - a.min_gap) % 360:.0f}-{(base_hue + a.min_gap) % 360:.0f}). "
              "If the user named a brand colour that lands inside the gate, pass --force.")
        return
    cmap = {}
    for h, (hu, s, l) in hsl.items():
        if s < 0.12:                                    # neutrals: keep them neutral, just a hint of the new hue
            ns, nhu = min(s + 0.03, 0.08), a.hue
        else:                                           # chromatics: same hue distances, rotated onto the new hue.
            ns = max(0.0, min(1.0, s * a.sat))          # tinted surfaces (low saturation) collapse onto the new hue
            off = (hu - base_hue + 180) % 360 - 180     # instead of being flung to its opposite side
            nhu = a.hue + off * a.spread * min(1.0, s / 0.35)
        target = 1 - _lum(h) if a.invert else _lum(h)
        new = _at_luminance(nhu, ns, target)
        new = _hexsafe(new)                             # six decimal digits would export as pure red
        if new != h: cmap[h] = new
    if not cmap: print('the derived palette is identical to the default one -- use the default: run init without '
                       '--map.'); return
    fails = []
    for f, src in srcs.items():
        for ln, c, b, fsz, r, need, lvl in _contrast_hits(_apply_cmap(src, cmap)):
            if lvl == 'FAIL': fails.append(f'{os.path.basename(f)} line {ln}: text #{c} on #{b}, {fsz}px, contrast {r:.2f} < 3.0')
    for h, n in sorted(cmap.items(), key=lambda kv: -counts[kv[0]]): print(f'  #{h} -> #{n}   (used {counts[h]}x)')
    if fails:
        print('\ncontrast FAIL on the derived palette:'); [print('  ' + f) for f in fails[:10]]
        sys.exit('\nnot written. With --invert this is expected for most sets: flipping light and dark cannot keep the\n'
                 'contrast the set was built with. Pick a set whose default scheme is already the one you want, or drop\n'
                 '--invert (a hue on its own always keeps the contrast). Otherwise try another --hue or --sat below 1.'
                 if a.invert else '\nnot written. Try another --hue, or --sat below 1.')
    try:
        _json.dump({'#' + k: '#' + v for k, v in cmap.items()}, open(a.out, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    except OSError as e:
        sys.exit(f'cannot write {a.out!r}: {e.strerror}.\n'
                 'Wrong location, not a broken environment -- write it inside the working directory\n'
                 'under the session output dir, then retry.')
    print(f'\ncontrast OK on every page of the set -> {a.out}\nnext: python3 "{{baseDir}}/scripts/build_page.py" init --set {a.set} --map {a.out}')

def _load_cmap(path):
    cmap = {k.strip().lstrip('#').upper(): v.strip().lstrip('#').upper() for k, v in json.load(open(path, encoding='utf-8')).items()}
    for k, v in cmap.items():
        if not re.fullmatch(r'[0-9A-F]{6}', k) or not re.fullmatch(r'[0-9A-F]{6}', v): sys.exit(f'bad colour in map: {k} -> {v} (use 6-digit hex, e.g. "#1C2644": "#F4F1EA")')
    return cmap

def _apply_cmap(text, cmap):
    """Replace every mapped colour in a single pass, so {"#A": "#B", "#B": "#A"} really swaps the two
    (replacing one entry after another would turn both into A and leave text the same colour as its background)."""
    if not cmap: return text
    rgb = {tuple(int(k[i:i + 2], 16) for i in (0, 2, 4)): v for k, v in cmap.items()}
    hexes = '|'.join(cmap)
    rgbas = '|'.join(r'%d\s*,\s*%d\s*,\s*%d\s*,' % t for t in rgb)
    rx = re.compile(r'#(' + hexes + r')(?![0-9A-Fa-f])|rgba\(\s*(?=' + rgbas + r')(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,', re.I)
    def sub(m):
        if m.group(1): return '#' + cmap[m.group(1).upper()]
        v = rgb[tuple(int(m.group(i)) for i in (2, 3, 4))]
        return 'rgba(%d, %d, %d,' % tuple(int(v[i:i + 2], 16) for i in (0, 2, 4))
    return rx.sub(sub, text)

def recolor(a):
    """Apply a colour map {old_hex: new_hex} to shell / template files and write the recoloured copies to --out-dir.
    Matches hex literals case-insensitively and the matching rgba(r, g, b, ...) forms. Colours not in the map are left as they are."""
    import os
    cmap = _load_cmap(a.map)
    os.makedirs(a.out_dir, exist_ok=True); unused = set(cmap); untouched = {}
    for p in a.files:
        src = open(p, encoding='utf-8').read()
        unused -= {old for old in cmap if re.search('#' + old + '(?![0-9A-Fa-f])', src, re.I)}
        out = _apply_cmap(src, cmap)
        for h in set(x.upper() for x in HEX.findall(out)):
            if h not in cmap.values(): untouched[h] = untouched.get(h, 0) + 1
        dst = os.path.join(a.out_dir, os.path.basename(p)); open(dst, 'w', encoding='utf-8').write(out); print(f'ok {dst}')
    if unused: print('map entries that matched nothing: ' + ', '.join('#' + u for u in sorted(unused)))
    if untouched: print('colours left unchanged (not in map): ' + ', '.join('#' + u for u in sorted(untouched)) + '  -> add them to the map if they belong to the old palette')
    print('next: run `contrast` on the recoloured pages before using them')

def init(a):
    """Copy a set's shells and template pages into the working dir in one go (optionally recoloured), so the agent never
    has to run cp against the skill directory."""
    import os, shutil
    root = _set_root(a.set)
    try:
        os.makedirs(a.out, exist_ok=True)
    except OSError as e:
        sys.exit(f'cannot create the working directory {a.out!r}: {e.strerror}.\n'
                 'This is the wrong location, not a broken environment -- put the working directory\n'
                 'under the session output dir (e.g. <writable>/temp/<session_id>/output/<name>) and retry.\n'
                 'Do not probe the filesystem for other writable paths.')
    srcs = [os.path.join(root, 'shell', f) for f in sorted(os.listdir(os.path.join(root, 'shell')))] + \
           [os.path.join(root, 'slides', f) for f in sorted(os.listdir(os.path.join(root, 'slides')))] + [os.path.join(root, 'DESIGN.preset.md')]
    cmap = _load_cmap(a.map) if a.map else {}
    for src in srcs:
        name = os.path.basename(src)
        if name.startswith('content'): name = '00_shell' + name[len('content'):]
        elif src.endswith('.slide'): name = 'ref_' + name
        text = _apply_cmap(open(src, encoding='utf-8').read(), cmap)
        open(os.path.join(a.out, name), 'w', encoding='utf-8').write(text); print(f'ok {os.path.normpath(os.path.join(a.out, name))}')
    print('\nthe files above are in the working directory: open and write them by these bare file names only, never by an absolute path')
    refs = [os.path.basename(s).replace('.slide', '') for s in srcs if s.endswith('.slide')]
    print('BEFORE WRITING EACH PAGE: open the ref_*.slide whose skeleton is closest to that page and rework it —\n'
          f'      keep its columns, card shapes and gaps, type scale and signature treatment, swap only the words and numbers.\n'
          f'      Available skeletons: {", ".join("ref_" + r for r in refs)}\n'
          '      A body written without one of these open only looks like the template in its header and footer.')
    if not cmap: print('NOTE: copied with the set\'s DEFAULT palette. That is correct in exactly two cases: the user asked\n'
                       '      for the default or a brand colour by name, or palette said the colour intent is within its\n'
                       '      hue gate of this set\'s own hue. Either way say so in DESIGN.md.\n'
                       f'      Otherwise derive one first: palette --set {a.set} --hue <0-360> [--sat 0.8], then rerun init with --map colors.json')

def buildall(a):
    """Build every NN_body.jsx in the working dir with its NN_vars.json, then run check + contrast (+ images) once."""
    import glob, os, types
    bodies = sorted(glob.glob(os.path.join(a.dir, '[0-9][0-9]_body*.jsx'))); bad = 0; built = []
    # bodies written with a slug, e.g. 03_definition_body.jsx (+ 03_definition_vars.json -> 03_definition_page.jsx)
    named = sorted(p for p in glob.glob(os.path.join(a.dir, '[0-9][0-9]_*_body*.jsx'))
                   if p not in bodies and not re.match(r'\d\d_body', os.path.basename(p)))
    for body in named:
        b = os.path.basename(body); stem, suffix = re.match(r'(\d\d_.+?)_body(.*)\.jsx$', b).groups()
        shell = os.path.join(a.dir, '00_shell' + suffix + '.jsx')
        if not os.path.exists(shell): shell = os.path.join(a.dir, '00_shell.jsx')
        out = os.path.join(a.dir, f'{stem}_page.jsx'); vars_ = os.path.join(a.dir, f'{stem}_vars.json')
        manifest = os.path.join(a.dir, 'images.json')
        ns = types.SimpleNamespace(shell=shell, body=body, vars=vars_ if os.path.exists(vars_) else None,
                                   manifest=manifest if os.path.exists(manifest) else None, out=out)
        try: build(ns); built.append(out)
        except SystemExit as e: bad += 1; print(f'FAIL  {body}: {e}')
    for body in bodies:
        nn = os.path.basename(body)[:2]; suffix = os.path.basename(body)[len(nn) + 5:-4]
        shell = os.path.join(a.dir, '00_shell' + suffix + '.jsx')
        if not os.path.exists(shell): shell = os.path.join(a.dir, '00_shell.jsx')
        out = os.path.join(a.dir, f'{nn}_page.jsx'); vars_ = os.path.join(a.dir, f'{nn}_vars.json')
        manifest = os.path.join(a.dir, 'images.json')
        ns = types.SimpleNamespace(shell=shell, body=body, vars=vars_ if os.path.exists(vars_) else None,
                                   manifest=manifest if os.path.exists(manifest) else None, out=out)
        try: build(ns); built.append(out)
        except SystemExit as e: bad += 1; print(f'FAIL  {body}: {e}')
    bodies = bodies + named
    pages = sorted(set(built + [p for p in glob.glob(os.path.join(a.dir, '[0-9][0-9]_*.jsx')) if not os.path.basename(p).startswith('00_')]) - set(bodies) - {p for p in glob.glob(os.path.join(a.dir, '[0-9][0-9]_*.jsx')) if re.search(r'_body[^/]*\.jsx$', p)})
    if not bodies and not pages:
        orphan = sorted(os.path.basename(p) for p in glob.glob(os.path.join(a.dir, '[0-9][0-9]_vars.json')))
        if orphan:
            sys.exit(f'nothing to build in {os.path.abspath(a.dir)}: found {", ".join(orphan)} but no matching body.\n'
                     'A page needs BOTH files: NN_body.jsx (or NN_body_light.jsx / NN_body_dark.jsx for a second\n'
                     'shell) plus NN_vars.json. Only the body file makes buildall pick the page up -- vars alone is\n'
                     'invisible to it. Write the missing NN_body*.jsx and rerun; the suffix must match the shell you\n'
                     'want (00_shell_dark.jsx <- NN_body_dark.jsx), no suffix falls back to 00_shell.jsx.')
        sys.exit(f'nothing to build in {os.path.abspath(a.dir)}: no NN_body*.jsx and no NN_*.jsx page found.\n'
                 'Run this command from the working directory that holds the page sources (or pass --dir), '
                 'otherwise it would report ALL OK without checking anything.')
    import io, contextlib
    steps = [('contrast (colours, svg, canvas edges)', contrast, types.SimpleNamespace(pages=pages)),
             ('fit (does the text fit its box)', fit, types.SimpleNamespace(pages=pages))]
    if os.path.exists(os.path.join(a.dir, 'images.json')):
        steps.append(('images (aspect ratio)', images, types.SimpleNamespace(manifest=os.path.join(a.dir, 'images.json'), pages=pages)))
    failed = {}
    for title, fn, ns in steps:
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try: fn(ns)
            except SystemExit as e: bad += bool(e.code)
        out = buf.getvalue()
        print(f'== {title} ==')
        print(out, end='')
        for ln in out.splitlines():
            m = re.match(r'(OVER|LOW|BAD|DRIFT)\s+(\S+)', ln)
            if m: failed.setdefault(os.path.basename(m.group(2)), []).append(m.group(1))
    if not bad:
        print('ALL OK'); sys.exit(0)
    print(f'\n{bad} problem(s) above. NOT OK -- these pages fail (a page is only OK when it passes EVERY section):')
    for pg, kinds in sorted(failed.items()): print(f'  {pg}: {", ".join(dict.fromkeys(kinds))}')
    print('Fix ALL of them in one pass -- read each OVER line, edit every page listed, and only then rerun\n'
          'buildall once; rerunning after each small edit burns a round trip per box. When a box is short, cut\n'
          'words or drop a block (the hint under each page says which); do not creep heights up a few px at a time.')
    print('Fix every page listed, whoever wrote it and whenever: pages submitted earlier, pages written in a\n'
          'previous plan step and cover / catalog / section / ending pages count too. Edit the source, rerun\n'
          'buildall until ALL OK. A page that is already in the document is redone as a whole: first\n'
          '`slide pages delete` at its page_index, then `slide pages addjsx` at that same page_index\n'
          '(addjsx INSERTS -- without the delete the page appears twice). Pages not submitted yet are just\n'
          'submitted as usual. Do not export or deliver while this list is not empty.')
    sys.exit(1)

# ---- fit: does the content fit the box it was written into? -----------------------------------
_FIT_TAG = re.compile(r"<(/?)([A-Za-z][A-Za-z0-9]*)((?:[^<>]|=>)*?)(/?)>", re.S)
_NUM = re.compile(r"^-?\d+(?:\.\d+)?$")
_LEAF = ('Text', 'Image', 'Chart', 'Table', 'FAIcon', 'QRCode', 'CodeBlock', 'Math', 'svg', 'br')

class _N:
    __slots__ = ('tag', 'st', 'line', 'lines', 'kids', 'raw')
    def __init__(s, tag, st, line): s.tag, s.st, s.line, s.lines, s.kids, s.raw = tag, st, line, [], [], False

def _style_dict(attrs):
    """One tag's inline style={{ ... }} as {property: raw value}."""
    m = re.search(r"style=\{\{", attrs)
    if not m: return {}
    i, depth = m.end(), 2
    while i < len(attrs) and depth:
        if attrs[i] == '{': depth += 1
        elif attrs[i] == '}': depth -= 1
        i += 1
    parts, cur, depth, q = [], '', 0, ''
    for ch in attrs[m.end():i - 2]:
        if q:
            cur += ch
            if ch == q: q = ''
            continue
        if ch in '"\'': q = ch; cur += ch; continue
        if ch in '([{': depth += 1
        elif ch in ')]}': depth -= 1
        if ch == ',' and depth == 0: parts.append(cur); cur = ''; continue
        cur += ch
    parts.append(cur)
    out = {}
    for p in parts:
        if ':' not in p: continue
        k, v = p.split(':', 1)
        out[k.strip().strip("'\"")] = v.strip()
    return out

def _px(v, base=None):
    """A style value in pixels: 12, '12px', or '50%' of base. None when it cannot be resolved."""
    if v is None: return None
    v = str(v).strip().strip("'\"")
    if _NUM.match(v): return float(v)
    if v.endswith('px') and _NUM.match(v[:-2]): return float(v[:-2])
    if v.endswith('%') and _NUM.match(v[:-1]): return None if base is None else float(v[:-1]) / 100 * base
    return None

def _edges(st, key):
    """(top, right, bottom, left) for padding / margin, from the shorthand and the per-side keys."""
    t = r = b = l = 0.0
    raw = st.get(key)
    if raw is not None:
        nums = [n for n in (_px(x) for x in str(raw).strip().strip("'\"").split()) if n is not None]
        if len(nums) == 1: t = r = b = l = nums[0]
        elif len(nums) == 2: t = b = nums[0]; r = l = nums[1]
        elif len(nums) == 3: t, r, b = nums[:3]; l = r
        elif len(nums) >= 4: t, r, b, l = nums[:4]
    for i, side in enumerate(('Top', 'Right', 'Bottom', 'Left')):
        v = _px(st.get(key + side))
        if v is None: continue
        t, r, b, l = (v, r, b, l) if i == 0 else (t, v, b, l) if i == 1 else (t, r, v, l) if i == 2 else (t, r, b, v)
    return t, r, b, l

def _border(st):
    def w(k):
        m = re.match(r"\s*'?\s*(\d+(?:\.\d+)?)px", str(st.get(k, '') or ''))
        return float(m.group(1)) if m else None
    a = w('border') or 0.0
    return tuple(w('border' + s) if w('border' + s) is not None else a for s in ('Top', 'Right', 'Bottom', 'Left'))

def _advance(ch, mono):
    """Advance width of one character in em — the model the presets quote for counting lines."""
    o = ord(ch)
    if o >= 0x1100 or 0x2010 <= o <= 0x203F: return 1.0
    if mono: return 0.6
    if ch == ' ': return 0.3
    if 'A' <= ch <= 'Z': return 0.75
    if 'a' <= ch <= 'z' or ch.isdigit(): return 0.55
    return 0.5

def _runs(inner):
    """A <Text>'s body as lines of (text, span style): <br /> breaks a line, spans keep their own size."""
    lines, cur, stack, i = [], [], [], 0
    def put(s):
        s = re.sub(r"^\s*\n\s*|\s*\n\s*$", '', s)            # newlines next to a tag vanish (JSX)
        s = re.sub(r"\s*\n\s*", ' ', s).replace('&nbsp;', ' ')  # a newline between words becomes ONE space (JSX)
        if s: cur.append((s, stack[-1] if stack else {}))
    while i < len(inner):
        m = re.compile(r"<(/?)([A-Za-z][A-Za-z0-9]*)?((?:[^<>]|=>)*?)(/?)>").search(inner, i)
        if not m: put(inner[i:]); break
        put(inner[i:m.start()])
        name, close, selfc = (m.group(2) or ''), m.group(1), m.group(4)
        if name == 'br': lines.append(cur); cur = []
        elif name == 'span' and not close and not selfc: stack.append(_style_dict(m.group(3)))
        elif name == 'span' and close and stack: stack.pop()
        i = m.end()
    lines.append(cur)
    return [ln for ln in lines if any(t.strip() for t, _ in ln)] or []

def _close_at(src, pos, name):
    """(inner end, resume) for the tag that was opened just before pos."""
    depth = 1
    for m in re.finditer(r"<(/?)" + name + r"\b((?:[^<>]|=>)*?)(/?)>", src[pos:], re.S):
        if m.group(1): depth -= 1
        elif not m.group(3): depth += 1
        if not depth: return pos + m.start(), pos + m.end()
    return len(src), len(src)

def _parse(src):
    root = _N('root', {}, 0); stack, i = [root], 0
    while True:
        m = _FIT_TAG.search(src, i)
        if not m: break
        close, name, attrs, selfc = m.group(1), m.group(2), m.group(3), m.group(4)
        if close:
            if len(stack) > 1 and stack[-1].tag == name: stack.pop()
            i = m.end(); continue
        n = _N(name, _style_dict(attrs), src.count('\n', 0, m.start()) + 1)
        stack[-1].kids.append(n)
        if name == 'Text' and not selfc:
            end, after = _close_at(src, m.end(), 'Text'); inner = src[m.end():end]
            n.lines = _runs(inner); n.raw = bool(re.search(r"[^\s<>]\s*\n\s*[^\s<>]", inner)); i = after; continue
        if name == 'svg' and not selfc:
            for k in ('width', 'height'):
                a = re.search(r"\b" + k + r"=[\"']?\{?(\d+(?:\.\d+)?)", attrs)
                if a and k not in n.st: n.st[k] = a.group(1)
            end, after = _close_at(src, m.end(), 'svg')
            n.lines = [attrs, src[m.end():end]]
            i = after; continue
        if not selfc and name not in _LEAF: stack.append(n)
        i = m.end()
    return root

def _inherit(st, parent):
    """Font context a node passes to its children: (fontSize, lineHeight, letterSpacing, family)."""
    fs, lh, ls, fam = parent
    return (_px(st.get('fontSize')) or fs, _px(st.get('lineHeight')) or lh,
            _px(st.get('letterSpacing')) if st.get('letterSpacing') is not None else ls,
            str(st.get('fontFamily') or fam).strip("'\""))

def _line_w(runs, ctx):
    fs, _lh, ls, fam = ctx
    w = 0.0
    for text, sst in runs:
        f = _px(sst.get('fontSize')) or fs
        sp = _px(sst.get('letterSpacing')) if sst.get('letterSpacing') is not None else ls
        mono = 'Courier' in str(sst.get('fontFamily') or fam)
        w += sum(_advance(c, mono) for c in text) * f + (sp or 0.0) * len(text)
    return w

def _flow(n): return [k for k in n.kids if str(k.st.get('position', '')).strip("'\"") != 'absolute']

def _max_w(n, ctx, avail=None):
    """Width the node wants when nothing forces it to wrap (flexbox max-content)."""
    st = n.st
    ex = _px(st.get('width'), avail)
    pt, pr, pb, pl = _edges(st, 'padding'); bt, br_, bb, bl = _border(st)
    ml = _edges(st, 'margin')[3] + _edges(st, 'margin')[1]
    if ex is not None: return ex + ml
    if n.tag == 'Text':
        c = _inherit(st, ctx)
        return (max([_line_w(l, c) for l in n.lines] or [0.0]) + pl + pr + bl + br_ + ml)
    if n.tag in _LEAF: return ml
    kids, c = _flow(n), _inherit(st, ctx)
    if not kids: return pl + pr + bl + br_ + ml
    inner = [_max_w(k, c) for k in kids]
    gap = _px(st.get('gap')) or 0.0
    room = sum(inner) + gap * (len(inner) - 1) if 'row' in str(st.get('flexDirection', '')) else max(inner)
    return room + pl + pr + bl + br_ + ml

def _min_w(n, ctx, avail=None):
    """Width below which the node cannot shrink (nowrap text cannot shrink at all)."""
    st = n.st
    ex = _px(st.get('width'), avail)
    if ex is not None: return ex
    if n.tag == 'Text':
        c = _inherit(st, ctx)
        if 'nowrap' in str(st.get('whiteSpace', '')): return _max_w(n, ctx)
        return max([max([_line_w([(w, s)], c) for w, s in l] or [0.0]) for l in n.lines] or [0.0]) * 0.12 + 1
    if n.tag in _LEAF: return 0.0
    kids, c = _flow(n), _inherit(st, ctx)
    if not kids: return 0.0
    pt, pr, pb, pl = _edges(st, 'padding'); bt, br_, bb, bl = _border(st)
    inner = [_min_w(k, c) for k in kids]
    return (sum(inner) if 'row' in str(st.get('flexDirection', '')) else max(inner)) + pl + pr + bl + br_

def _row_widths(kids, cw, gap, ctx):
    """Resolve a row's child widths the way flexbox does: explicit, then flex share, then max-content shrunk to fit."""
    n = len(kids)
    basis, grow, fixed = [], [], []
    for k in kids:
        ex = _px(k.st.get('width'), cw)
        fl = _px(str(k.st.get('flex', '')).split()[0]) if k.st.get('flex') is not None else None
        basis.append(ex if ex is not None else (0.0 if fl is not None else _max_w(k, ctx)))
        grow.append(fl); fixed.append(ex is not None)
    free = cw - gap * max(0, n - 1) - sum(basis)
    pool = [i for i, g in enumerate(grow) if g is not None]
    out = list(basis)
    if pool and free > 0:
        units = sum(grow[i] or 1.0 for i in pool) or 1.0
        for i in pool: out[i] += free * (grow[i] or 1.0) / units
    elif free < 0:
        shrink = [i for i in range(n) if not fixed[i]] or list(range(n))
        total = sum(basis[i] for i in shrink) or 1.0
        for i in shrink:
            out[i] = max(_min_w(kids[i], ctx), basis[i] + free * basis[i] / total)
    return out

def _fit(n, avail_w, avail_h, ctx, hits, sized=True):
    """Lay the node out in (avail_w, avail_h); return the height it actually needs.
    `sized`: the node has a definite width (its own, a stretched column parent's, or a flex share). Text inside an
    auto-width container is measured by the WeCom compiler at its one-line width: if that is wider than the room the
    row leaves it, the box is placed as ONE line but drawn wrapped, so it runs down behind the block below."""
    import math
    st = n.st
    if _px(st.get('width'), avail_w) is not None: sized = True
    pt, pr, pb, pl = _edges(st, 'padding'); bt, br_, bb, bl = _border(st)
    mt, mr, mb, ml = _edges(st, 'margin')
    absolute = str(st.get('position', '')).strip("'\"") == 'absolute'
    w = _px(st.get('width'), avail_w)
    if w is None and not absolute: w = None if avail_w is None else avail_w - ml - mr
    if w is None and absolute and avail_w is not None:
        lft, rgt = _px(st.get('left'), avail_w), _px(st.get('right'), avail_w)
        if lft is not None and rgt is not None: w = avail_w - lft - rgt
    cw = None if w is None else max(0.0, w - pl - pr - bl - br_)
    h = _px(st.get('height'), avail_h)
    ch = None if h is None else max(0.0, h - pt - pb - bt - bb)
    c = _inherit(st, ctx)

    if n.tag == 'Text':
        fs, lh, _ls, _fam = c
        if lh > 4: lh = lh / fs
        nowrap = 'nowrap' in str(st.get('whiteSpace', ''))
        rows, widest = 0, 0.0
        for ln in n.lines:
            wide = _line_w(ln, c); widest = max(widest, wide)
            rows += 1 if (nowrap or not cw) else max(1, math.ceil(wide / cw - 1e-6))
        if nowrap and cw and widest > cw + 1:
            hits.append((n.line, f'{fs:.0f}px nowrap text needs {widest:.0f}px, its column is {cw:.0f}px wide', widest - cw))
        elif not sized and cw and widest > cw + 1:
            hits.append((n.line, f'{fs:.0f}px text in a container with no fixed width needs {widest:.0f}px on one line but only '
                         f'{cw:.0f}px is left in the row: the compiler places it as ONE line and it runs down behind the block '
                         f'below -- break it with <br /> so every line fits, or give the container a width', widest - cw))
        if n.raw:
            hits.append((n.line, 'a raw line break inside <Text>: JSX turns it into a space, not a new line -- write <br />', 0.0))
        return rows * fs * lh * 1.2 + pt + pb + bt + bb + mt + mb   # the compiler's line box is 1.2 x the declared lineHeight (measured on exports: 76px/1.1 -> 100.5px, 15px/1.55 -> 28px, no lineHeight -> 1.2em)

    if n.tag in _LEAF: return (h if h is not None else 0.0) + mt + mb

    kids = _flow(n)
    row = 'row' in str(st.get('flexDirection', ''))
    gap = _px(st.get('gap')) or 0.0
    widths = _row_widths(kids, cw, gap, c) if (row and cw is not None) else [None if cw is None else cw] * len(kids)
    heights = []
    stretch = str(st.get('alignItems', 'stretch')).strip("'\"") == 'stretch'
    for k, kw in zip(kids, widths):
        ks = sized and (k.st.get('flex') is not None if row else stretch)   # own width is picked up inside _fit
        need = _fit(k, kw, ch, c, hits, ks)
        fix = _px(k.st.get('height'), ch)
        km = _edges(k.st, 'margin')
        heights.append((fix + km[0] + km[2]) if fix is not None else need)
    for k in n.kids:
        if str(k.st.get('position', '')).strip("'\"") == 'absolute':
            _fit(k, cw, ch, c, hits, k.st.get('left') is not None and k.st.get('right') is not None)
    inner = (max(heights) if row else sum(heights) + gap * (len(heights) - 1)) if heights else 0.0
    need = inner + pt + pb + bt + bb
    if h is not None and need > h + max(2.0, 0.015 * h):
        hits.append((n.line, f'the children need {need:.0f}px, this box is {h:.0f}px tall', need - h))
    return (h if h is not None else need) + mt + mb

_ESC_NL = re.compile(r"""(['"])(?:[^'"\\\n]|\\.)*?\\n(?:[^'"\\\n]|\\.)*?\1""")

def _escaped_nl_hits(src):
    r"""String literals holding a \n escape. The compiler renders it as a real line break, but the height
    estimate only sees the interpolation ({d[1]}) or one joined line, so such a box silently comes out
    taller than measured and runs over whatever sits below it. Write <br /> instead (or split the data)."""
    hits = []
    for m in _ESC_NL.finditer(src):
        ln = src.count('\n', 0, m.start()) + 1
        txt = m.group(0)
        n = txt.count('\\n')
        hits.append((ln, rf"a \n escape inside a string literal ({n} of them): the compiler draws it as a real "
                         rf"line break, but the height check counts ONE line -- the box renders taller than "
                         rf"measured and overlaps the block below. Use <br /> in the <Text>, or split the value "
                         rf"into separate <Text> elements: {txt[:58]}", 0.0))
    return hits

_BR_IN_STR = re.compile(r"""(['"])(?:[^'"\\\n]|\\.)*?<br\s*/?>(?:[^'"\\\n]|\\.)*?\1""")

def _in_jsx_text(src, pos):
    """True when pos sits in JSX text (child text of a tag), not inside a {...} JS expression. A quote mark
    there is a literal character, so a '<br />' next to it is a real JSX line break, not text in a string."""
    depth, q, i = 0, '', 0
    while i < pos:
        ch = src[i]
        if q:
            if ch == '\\': i += 2; continue
            if ch == q: q = ''
        elif depth and ch in '\'"`': q = ch
        elif ch == '{': depth += 1
        elif ch == '}': depth = max(0, depth - 1)
        i += 1
    if depth: return False
    k = pos - 1
    while k >= 0 and src[k] not in '<>': k -= 1
    return k >= 0 and src[k] == '>'           # last tag bracket before pos is a closing '>': we are in text

def _br_in_string_hits(src):
    r"""String literals holding a <br /> tag. <br /> only works in JSX text position (a direct child of
    <Text>); inside a data-array string interpolated via {d[i]} it is inserted as PLAIN TEXT and the
    six characters render verbatim on the page. Split the value into separate <Text> elements, or drop
    the array and write the JSX out."""
    hits = []
    for m in _BR_IN_STR.finditer(src):
        br = src.find('<br', m.start())
        if br >= 0 and _in_jsx_text(src, br): continue   # the <br /> is JSX text (quotes around it are literal characters): it works
        ln = src.count('\n', 0, m.start()) + 1
        txt = m.group(0)
        hits.append((ln, f"a <br /> tag inside a string literal: interpolated strings are inserted as plain "
                         f"text, so the tag renders verbatim on the page -- split the value into separate "
                         f"<Text> elements or write the JSX out instead of a data array: {txt[:58]}", 0.0))
    return hits

# ---- layer: absolutely positioned text vs. flow-laid-out content ---------------------------------
# The flow height check (_fit) and the canvas check (_abs_hits) each look at ONE coordinate system. A block
# pinned with bottom:/top: is placed without knowing where the flow actually put the title above it, so the
# two can land on the same pixels and both checks still say OK (seen on section pages: a lead-in pinned at
# bottom:158 sat right on top of a title the flow had pushed down to y 484). This pass lays the page out with
# real coordinates and reports every absolute text box that intersects FLOW text. Left alone on purpose: text over
# an svg / image backdrop and absolute blocks overlapping each other (Venn diagrams, badges). {[...].map(...)}
# over a literal array is unrolled first; a .map the parser cannot unroll makes it skip the page, not guess.
_VIS = ('Image', 'Chart', 'Table', 'svg', 'CodeBlock', 'Math', 'QRCode')

def _is_abs(n): return str(n.st.get('position', '')).strip("'\"") == 'absolute'

def _text_w(n, ctx, cw):
    c = _inherit(n.st, ctx)
    pl, pr = _edges(n.st, 'padding')[3], _edges(n.st, 'padding')[1]
    ex = _px(n.st.get('width'), cw)
    if ex is not None: return ex
    want = max([_line_w(l, c) for l in n.lines] or [0.0]) + pl + pr
    return want if cw is None else min(cw, want)

def _place(n, x, y, avail_w, avail_h, ctx, sized, layer, out):
    """Lay n out with its border box at (x, y); append (x0, y0, x1, y1, line, layer, kind, fontSize) for every
    Text / visual leaf. `layer` is None for flow content, otherwise the line of the absolute ancestor."""
    st = n.st
    if _px(st.get('width'), avail_w) is not None: sized = True
    pt, pr, pb, pl = _edges(st, 'padding'); bt, br_, bb, bl = _border(st)
    mt, mr, mb, ml = _edges(st, 'margin')
    w = _px(st.get('width'), avail_w)
    if w is None and not _is_abs(n) and avail_w is not None: w = avail_w - ml - mr
    c = _inherit(st, ctx)
    need = _fit(n, avail_w, avail_h, ctx, [], sized) - mt - mb
    h = _px(st.get('height'), avail_h)
    h = need if h is None else h
    if n.tag == 'Text':
        tw = _text_w(n, ctx, w if w is not None else avail_w)
        op = _px(st.get('opacity'))
        if not (c[0] >= 150 or (op is not None and op < 0.5)):   # watermark numerals / ghosted text are decoration
            out.append((x, y, x + tw, y + h, n.line, layer, 'Text', c[0]))
        return
    if n.tag in _LEAF:
        if n.tag in _VIS and w is not None: out.append((x, y, x + w, y + h, n.line, layer, n.tag, 0.0))
        if n.tag == 'svg' and layer is not None and w and h and len(n.lines) == 2:
            for sx0, sy0, sx1, sy1 in _svg_shapes(n.lines[0], n.lines[1], w, h):
                out.append((x + sx0, y + sy0, x + sx1, y + sy1, n.line, layer, 'deco', 0.0))
        return
    cw = None if w is None else max(0.0, w - pl - pr - bl - br_)
    ch = max(0.0, h - pt - pb - bt - bb)
    ox, oy = x + bl + pl, y + bt + pt
    kids = _flow(n)
    row = 'row' in str(st.get('flexDirection', ''))
    gap = _px(st.get('gap')) or 0.0
    widths = _row_widths(kids, cw, gap, c) if (row and cw is not None) else [cw] * len(kids)
    stretch = str(st.get('alignItems', 'stretch')).strip("'\"") == 'stretch'
    align = str(st.get('alignItems', 'stretch')).strip("'\"")
    just = str(st.get('justifyContent', 'flex-start')).strip("'\"")
    sizes = []
    for k, kw in zip(kids, widths):
        ks = sized and (k.st.get('flex') is not None if row else stretch)
        km = _edges(k.st, 'margin')
        fix = _px(k.st.get('height'), ch)
        kh = fix if fix is not None else _fit(k, kw, ch, c, [], ks) - km[0] - km[2]
        if row: kwid = kw if kw is not None else 0.0
        elif k.tag == 'Text': kwid = _text_w(k, c, kw)
        else:
            kwid = _px(k.st.get('width'), cw)
            kwid = kwid if kwid is not None else ((kw - km[1] - km[3]) if (kw is not None and stretch) else (_max_w(k, c) - km[1] - km[3]))
        sizes.append((kwid, kh, km, ks))
    main = [(s[0] + s[2][1] + s[2][3]) if row else (s[1] + s[2][0] + s[2][2]) for s in sizes]
    room = (cw if row else ch) or 0.0
    free = room - sum(main) - gap * max(0, len(main) - 1)
    lead, step = 0.0, gap
    if free > 0 and (row or _px(st.get('height'), avail_h) is not None):
        if just == 'center': lead = free / 2
        elif just == 'flex-end': lead = free
        elif just == 'space-between' and len(main) > 1: step = gap + free / (len(main) - 1)
        elif just == 'space-around' and main: lead = free / len(main) / 2; step = gap + free / len(main)
    cur = lead
    for k, kw, (kwid, kh, km, ks) in zip(kids, widths, sizes):
        if row:
            cross = ch if ch else max([s[1] + s[2][0] + s[2][2] for s in sizes] or [0.0])
            off = (cross - kh - km[0] - km[2]) / 2 if align == 'center' else (cross - kh - km[0] - km[2]) if align == 'flex-end' else 0.0
            _place(k, ox + cur + km[3], oy + max(0.0, off) + km[0], kw, ch, c, ks, layer, out)
            cur += kwid + km[1] + km[3] + step
        else:
            off = ((cw or 0) - kwid - km[1] - km[3]) / 2 if align == 'center' else ((cw or 0) - kwid - km[1] - km[3]) if align == 'flex-end' else 0.0
            _place(k, ox + max(0.0, off) + km[3], oy + cur + km[0], kw, ch, c, ks, layer, out)
            cur += kh + km[0] + km[2] + step
    pw = w if w is not None else (avail_w or 0.0)
    for k in n.kids:
        if not _is_abs(k): continue
        ks_ = k.st
        kw_ = _px(ks_.get('width'), pw)
        lft, rgt = _px(ks_.get('left'), pw), _px(ks_.get('right'), pw)
        if kw_ is None and lft is not None and rgt is not None: kw_ = pw - lft - rgt
        if kw_ is None: kw_ = _max_w(k, c)
        kh_ = _px(ks_.get('height'), h)
        if kh_ is None: kh_ = _fit(k, kw_, h, c, [], True)
        top, bot = _px(ks_.get('top'), h), _px(ks_.get('bottom'), h)
        ax = x + (lft if lft is not None else (pw - rgt - kw_) if rgt is not None else bl + pl)
        ay = y + (top if top is not None else (h - bot - kh_) if bot is not None else bt + pt)
        _place(k, ax, ay, kw_, kh_, c, True, layer if layer is not None else k.line, out)

def _match(src, i, o, c):
    """Index just past the bracket that closes src[i] (== o), skipping string literals."""
    depth, q, j = 0, '', i
    while j < len(src):
        ch = src[j]
        if q:
            if ch == '\\': j += 2; continue
            if ch == q: q = ''
        elif ch in '\'"`': q = ch
        elif ch == o: depth += 1
        elif ch == c:
            depth -= 1
            if not depth: return j + 1
        j += 1
    return -1

def _split_top(s):
    out, cur, depth, q = [], '', 0, ''
    for ch in s:
        if q:
            cur += ch
            if ch == q: q = ''
            continue
        if ch in '\'"`': q = ch; cur += ch; continue
        if ch in '([{': depth += 1
        elif ch in ')]}': depth -= 1
        if ch == ',' and depth == 0: out.append(cur); cur = ''; continue
        cur += ch
    if cur.strip(): out.append(cur)
    return [x.strip() for x in out if x.strip()]

def _lit(v):
    v = v.strip()
    return v[1:-1] if len(v) >= 2 and v[0] == v[-1] and v[0] in '\'"`' else None

_MAP = re.compile(r"\{\s*\[")
def _expand_maps(src):
    """Unroll {[literal, ...].map((d, i) => ( <JSX/> ))} into N copies of the JSX with d[k] / d.key / i filled
    in, so the layout pass sees the real row count and the real strings. Returns None when a .map cannot be
    unrolled (variable array, nested function body) -- the caller then skips the page instead of guessing."""
    for _ in range(50):
        m = _MAP.search(src)
        if not m: break
        a0 = m.end() - 1; a1 = _match(src, a0, '[', ']')
        if a1 < 0: return None
        mm = re.match(r"\s*\.map\(\s*\(?\s*(\w+)\s*(?:,\s*(\w+)\s*)?\)?\s*=>\s*\(", src[a1:])
        if not mm: return None
        b0 = a1 + mm.end() - 1; b1 = _match(src, b0, '(', ')')
        if b1 < 0: return None
        tail = re.match(r"\s*\)\s*\}", src[b1:])
        if not tail: return None
        var, idx, body = mm.group(1), mm.group(2), src[b0 + 1:b1 - 1]
        rows = []
        for k, el in enumerate(_split_top(src[a0 + 1:a1 - 1])):
            vals = {}
            if el.startswith('['):
                for n, v in enumerate(_split_top(el[1:-1])): vals[f'{var}[{n}]'] = _lit(v)
            elif el.startswith('{'):
                for kv in _split_top(el[1:-1]):
                    if ':' in kv:
                        kk, vv = kv.split(':', 1); vals[f'{var}.{kk.strip().strip(chr(39) + chr(34))}'] = _lit(vv)
            else: vals[var] = _lit(el)
            b = body
            if idx: b = re.sub(r"\b" + idx + r"\b(?=\s*(?:[=!<>]=?=?|\)|\}|\+|-|\*))", str(k), b)
            b = re.sub(r"\{\s*" + re.escape(var) + r"(\[\d+\]|\.\w+)?\s*\}",
                       lambda q: (vals.get(var + (q.group(1) or '')) or 'X' * 6).replace('<', '&lt;'), b)
            rows.append(b)
        src = src[:m.start()] + '\n'.join(rows) + src[b1 + tail.end():]
    if '.map(' in src or '=>' in src: return None
    # leftover JS the tag parser cannot read (ternaries, comparisons, method calls) -> the layout would be a guess
    if re.search(r"\?[^{}]*:|\b\d+\s*[<>]=?\s*\d+|\{[^{}]*\.\w+\([^{}]*\}", src): return None
    return src

_SHAPE = re.compile(r"<(rect|circle|ellipse)\b([^>]*)/?>")
def _svg_shapes(attrs, inner, w, h):
    """Solid decoration shapes inside an absolutely positioned svg, in page pixels relative to the svg box.
    Kept: rect / circle / ellipse that are filled (colour or pattern), at least 24px on both sides and cover
    less than half of the svg -- i.e. a corner mark, a bar, a stripe block. Dropped: backdrop rects, hairlines,
    sparkle dots, outline-only rings, faint (< 0.3 opacity) shapes, anything inside <defs> / <pattern>."""
    vb = re.search(r"viewBox=['\"]\s*([-\d.]+)[\s,]+([-\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)", attrs)
    vx, vy, vw, vh = (float(v) for v in vb.groups()) if vb else (0.0, 0.0, w, h)
    if not vw or not vh: return []
    kx, ky = w / vw, h / vh
    sop = re.search(r"\bopacity:\s*([\d.]+)", attrs)
    if sop and float(sop.group(1)) < 0.3: return []                  # the whole svg is a faint watermark
    inner = re.sub(r"<g\b[^>]*\bfill=['\"]none['\"][^>]*>.*?</g>", '', inner, flags=re.S)   # outline-only groups
    inner = re.sub(r"<defs\b.*?</defs>|<pattern\b.*?</pattern>|<clipPath\b.*?</clipPath>|<mask\b.*?</mask>", '', inner, flags=re.S)
    if re.search(r"<g\b[^>]*transform", inner): return []          # moved groups: geometry is not where it is written
    out = []
    for m in _SHAPE.finditer(inner):
        tag, a = m.group(1), m.group(2)
        if re.search(r"\b(?:x|y|width|height|cx|cy|r|rx|ry)=['\"][^'\"]*%", a): continue   # percent geometry
        def f(k, d=None):
            q = re.search(r"\b" + k + r"=['\"]\{?\s*([-\d.]+)", a)
            return float(q.group(1)) if q else d
        fill = re.search(r"\bfill=['\"]([^'\"]*)", a)
        if (fill and fill.group(1).strip().lower() in ('none', 'transparent')) or 'transform' in a: continue
        op = f('opacity', 1.0); fo = f('fillOpacity', 1.0)
        if op * fo < 0.3: continue
        if tag == 'rect':
            x0, y0, ww, hh = f('x', 0.0), f('y', 0.0), f('width'), f('height')
            if ww is None or hh is None: continue
        else:
            cx, cy = f('cx', 0.0), f('cy', 0.0)
            rx = f('r') if tag == 'circle' else f('rx'); ry = f('r') if tag == 'circle' else f('ry')
            if rx is None or ry is None: continue
            x0, y0, ww, hh = cx - rx, cy - ry, 2 * rx, 2 * ry
        if ww * hh >= 0.5 * vw * vh: continue                      # backdrop
        if ww * kx < 24 or hh * ky < 24: continue                   # hairline / dot
        out.append(((x0 - vx) * kx, (y0 - vy) * ky, (x0 - vx + ww) * kx, (y0 - vy + hh) * ky))
    return out

def _layer_hits(src):
    hits = []
    if '.map(' in src or '=>' in src:
        src = _expand_maps(src)
        if src is None: return hits           # data the static parser cannot unroll: stay silent rather than guess
    for top in _parse(src).kids:
        if top.tag != 'Slide': continue
        boxes = []
        _place(top, 0.0, 0.0, _px(top.st.get('width')) or 1280.0, _px(top.st.get('height')) or 720.0,
               (16.0, 1.0, 0.0, ''), True, None, boxes)
        seen = set()
        for i, a in enumerate(boxes):
            for b in boxes[i + 1:]:
                if a[5] == b[5]: continue                      # same layer: the flow keeps them apart
                if 'deco' in (a[6], b[6]):
                    d_, t_ = (a, b) if a[6] == 'deco' else (b, a)
                    if t_[6] != 'Text' or t_[5] is not None: continue   # only flow text is checked against decoration
                    dx = min(a[2], b[2]) - max(a[0], b[0]); dy = min(a[3], b[3]) - max(a[1], b[1])
                    if dx < 8 or dy < 4 or (d_[4], t_[4]) in seen: continue
                    seen.add((d_[4], t_[4]))
                    hits.append((t_[4], f"{t_[7]:.0f}px text at x {t_[0]:.0f}-{t_[2]:.0f} y {t_[1]:.0f}-{t_[3]:.0f} runs {dx:.0f}px into the "
                                 f"decoration drawn by the svg of line {d_[4]} (a shape at x {d_[0]:.0f}-{d_[2]:.0f} y {d_[1]:.0f}-{d_[3]:.0f}). "
                                 f"Keep text left of it: give the text a width (maxWidth is ignored by the compiler) or break it with <br />", dx))
                    continue
                if a[5] is not None and b[5] is not None: continue   # two absolute blocks (Venn circles, badges) overlap on purpose
                if a[6] != 'Text' or b[6] != 'Text': continue   # svg / picture under text is a backdrop by design; text on text never is
                ab, fl = (a, b) if a[5] is not None and (b[5] is None or a[5] > b[5]) else (b, a)
                dx = min(a[2], b[2]) - max(a[0], b[0]); dy = min(a[3], b[3]) - max(a[1], b[1])
                if dx < 4 or dy < 4: continue
                key = (ab[4], fl[4])
                if key in seen: continue
                seen.add(key)
                other = 'flow' if fl[5] is None else f'absolute (line {fl[5]})'
                hits.append((ab[4], f"absolutely positioned {ab[6]} at y {ab[1]:.0f}-{ab[3]:.0f} x {ab[0]:.0f}-{ab[2]:.0f} lands on "
                             f"the {other} {fl[6]} of line {fl[4]} (y {fl[1]:.0f}-{fl[3]:.0f}): {dy:.0f}px of the two overlap. "
                             f"Absolute offsets do not move when the flow above grows -- put this block in the flow "
                             f"(drop position/top/bottom, give it a marginTop) or delete it; nudging bottom/top a few px is not a fix, "
                             f"the next longer title lands on it again", dy))
    return hits

def _slide_size_hits(src):
    hits = []
    for top in _parse(src).kids:
        if top.tag != 'Slide': continue
        w, h = _px(top.st.get('width')) or 1280.0, _px(top.st.get('height')) or 720.0   # unset = the 1280x720 default
        if (w, h) != (1280.0, 720.0):
            hits.append((top.line, f"<Slide> is {w:.0f}x{h:.0f}: every page must be exactly 1280x720 (and keep the shell's padding) "
                                   f"-- another size is stretched on export and its coordinates no longer line up with the other pages", 0.0))
    return hits

_BARE_LT = re.compile(r"<(?![A-Za-z/>!])")
def _bare_lt_hits(src):
    """A '<' in JSX text that does not open a tag ('<500ms', '< 5%', '<=') -- the compiler reads it as the start
    of an element and addjsx fails with JSX_PARSE_ERROR after the page has already been submitted."""
    hits = []
    for m in _BARE_LT.finditer(src):
        if not _in_jsx_text(src, m.start()): continue
        ln = src.count('\n', 0, m.start()) + 1
        hits.append((ln, f"a bare '<' in text ({src[m.start():m.start() + 12].splitlines()[0]!r}) is parsed as a tag and "
                         f"addjsx fails with JSX_PARSE_ERROR -- write {{'<'}} or reword it (低于 / 小于)", 0.0))
    return hits

def _iter_nodes(n):
    for k in n.kids:
        yield k
        yield from _iter_nodes(k)

def _loose_rows(n, cw, ctx, hits):
    """Rows whose children all carry a hard-coded width and together leave most of the row unused."""
    st = n.st
    pr, pl = _edges(st, 'padding')[1], _edges(st, 'padding')[3]
    br_, bl = _border(st)[1], _border(st)[3]
    w = _px(st.get('width'), cw)
    inner = None if (w if w is not None else cw) is None else max(0.0, (w if w is not None else cw) - pl - pr - bl - br_)
    c = _inherit(st, ctx); kids = _flow(n); gap = _px(st.get('gap')) or 0.0
    row = 'row' in str(st.get('flexDirection', ''))
    if row and inner and len(kids) >= 2 and all(_px(k.st.get('width'), inner) is not None for k in kids):
        used = sum(_px(k.st.get('width'), inner) for k in kids) + gap * (len(kids) - 1)
        if used < 0.86 * inner and inner - used > 90:
            hits.append((n.line, f'this row of {len(kids)} blocks has every width hard-coded: {used:.0f}px used of the '
                                 f'{inner:.0f}px row, so {inner - used:.0f}px stays empty at the right. Replace the fixed '
                                 f"`width:` on each block with `flex: 1` so the row fills the region", 0.0))
    ws = _row_widths(kids, inner, gap, c) if (row and inner is not None) else [inner] * len(kids)
    for k, kw in zip(kids, ws): _loose_rows(k, kw, c, hits)
    for k in n.kids:
        if str(k.st.get('position', '')).strip("'\"") == 'absolute': _loose_rows(k, inner, c, hits)

def _slack_hits(src):
    """The mirror image of the fit check: content that falls far SHORT of the region it was given. A body region
    1216x551 holding blocks that stop at 780px wide and 327px tall renders as a page with its lower right third
    blank -- every box passes fit and contrast on its own, so nothing downstream ever sees it. Only regions big
    enough to be a body are looked at, and only when they hold a real set of blocks, so a cover or a section
    divider (a few lines on purpose) is never flagged."""
    hits = []
    for top in _parse(src).kids:
        if top.tag != 'Slide': continue
        ctx = (16.0, 1.0, 0.0, '')
        for box in _iter_nodes(top):
            if box.tag != 'Box' or str(box.st.get('position', '')).strip("'\"") != 'absolute': continue
            w, h = _px(box.st.get('width'), 1280.0), _px(box.st.get('height'), 720.0)
            if not w or not h or w < 560 or h < 240: continue
            if sum(1 for k in _iter_nodes(box) if k.tag == 'Text') < 5: continue
            if not any(k.tag == 'Box' for k in _flow(box)): continue
            keep = box.st.get('height')
            box.st.pop('height', None)
            try: need = _fit(box, w, h, ctx, [])
            finally:
                if keep is not None: box.st['height'] = keep
            if h - need > max(70.0, 0.22 * h):
                hits.append((box.line, f'the blocks in this {w:.0f}x{h:.0f} region add up to {need:.0f}px tall, leaving '
                                       f'{h - need:.0f}px ({(h - need) / h:.0%}) of it empty at the bottom. Give the blocks '
                                       f'real height (taller padding, one more row of substance, a bigger lead paragraph) '
                                       f'or move a block here from another page -- do NOT just stretch a box with a fixed '
                                       f'height, and do not pad it with filler sentences', 0.0))
            _loose_rows(box, w, ctx, hits)
    return hits

def _fit_hits(src):
    hits = []
    for top in _parse(src).kids:
        if top.tag != 'Slide': continue
        _fit(top, _px(top.st.get('width')) or 1280.0, _px(top.st.get('height')) or 720.0,
             (16.0, 1.0, 0.0, ''), hits)
    hits += _slide_size_hits(src)
    hits += _slack_hits(src)
    hits += _layer_hits(src)
    hits += _escaped_nl_hits(src)
    hits += _br_in_string_hits(src)
    hits += _bare_lt_hits(src)
    return sorted(hits, key=lambda x: -x[2])

def fit(a):
    """Flag content that does not fit its container: text whose wrapped height is taller than the fixed-height
    box around it, nowrap text wider than its column, and any fixed-height box — up to the 720px Slide — whose
    children add up to more than it holds. Sizes are estimated from the type metrics the presets use."""
    bad = 0
    for p in a.pages:
        hits = _fit_hits(open(p, encoding='utf-8').read())
        print(('OVER  ' if hits else 'OK    ') + p)
        for ln, msg, over in hits[:12]:
            print(f'      line {ln}: {msg}' + (f', over by {over:.0f}px' if over > 0 else ''))
        if any('lands on the' in m for _, m, _o in hits):
            print('      -> overlap: keep all text of a page in the flow (only decoration, footer and page number may be absolute); '
                  'a lead-in under a section / cover title goes right after the title with a marginTop, not pinned with bottom:')
        if any('stays empty' in m or 'empty at the bottom' in m for _, m, _o in hits):
            print('      -> this page is too EMPTY, not too full: fill the region instead of trimming. Fix it in one pass '
                  '(flex: 1 on the row, more substance per block) -- nudging sizes a few px at a time will not close it')
        if any('lands on the' not in m and 'empty' not in m for _, m, _o in hits):
            print('      -> cut the words, drop a block or split the page; do not shrink the type or edit the shell')
        bad += bool(hits)
    sys.exit(1 if bad else 0)

# ---------------------------------------------------------------- template: build the deck's shells FROM a user's PPT template
# A user who sends a template wants the deck to look like it. The pool sets cannot do that, so this path reads the
# template's own masters and layouts and turns them into shells: decorations (shapes, EMF art, small pictures) are
# rasterised into one transparent layer per page type, full-bleed pictures stay pictures, placeholders become 〔…〕
# slots at the template's own positions, sizes, colours and fonts. Standard library only (Pillow / sips optional).
_TNS = {'a': 'http://schemas.openxmlformats.org/drawingml/2006/main',
        'p': 'http://schemas.openxmlformats.org/presentationml/2006/main',
        'r': 'http://schemas.openxmlformats.org/officeDocument/2006/relationships',
        'rel': 'http://schemas.openxmlformats.org/package/2006/relationships'}
_T_RID = '{%s}id' % _TNS['r']; _T_EMBED = '{%s}embed' % _TNS['r']
_T_ALIAS = {'tx1': 'dk1', 'bg1': 'lt1', 'tx2': 'dk2', 'bg2': 'lt2'}
_T_SERIF = re.compile(r'宋|明朝|明體|仿宋|楷|隶|魏碑|serif(?!.*sans)|song|ming|mincho|kai|fangsong|times|georgia|garamond|'
                      r'cambria|palatino|baskerville|bodoni|didot|playfair|caslon|minion|book antiqua|century schoolbook|'
                      r'noto serif|source han serif|思源宋', re.I)
_T_MONO = re.compile(r'mono|courier|consolas|menlo|code', re.I)
_T_FILLER = re.compile(r'单击此处|点击此处|单击添加|点击添加|点击输入|单击输入|添加标题|输入标题|添加文字|添加文本|添加章节|'
                       r'请在此|在此输入|此处输入|此处添加|请输入|请替换|替换为您|输入您的|您的内容|您的标题|标题文字|示例文字|'
                       r'内容标题|建议\s*\d+\s*字|文字内容|内容填充|XX公司|XXX|20XX|\d{4}\.XX|XX\.XX|click to (?:add|edit)|'
                       r'lorem ipsum|your (?:title|text|company|logo)|add (?:your )?(?:title|text)|text here|title here|sample text', re.I)
# fontFamily whitelist of the WeCom renderer (design-principle.md §5) and the names templates use for them
_T_FONTS = {'microsoft yahei': 'Microsoft YaHei', '微软雅黑': 'Microsoft YaHei', 'microsoft yahei ui': 'Microsoft YaHei',
            'pingfang sc': 'PingFang SC', '苹方': 'PingFang SC', 'pingfang': 'PingFang SC', 'hiragino sans gb': 'Hiragino Sans GB',
            '冬青黑体简体中文': 'Hiragino Sans GB', 'simhei': 'SimHei', '黑体': 'SimHei', 'tencentsans w3': 'TencentSans W3',
            'tencentsans w7': 'TencentSans W7', 'tencent sans w3': 'TencentSans W3', 'tencent sans w7': 'TencentSans W7',
            '腾讯体 w3': 'TencentSans W3', '腾讯体 w7': 'TencentSans W7', '腾讯体': 'TencentSans W7', 'stsong': 'STSong',
            '华文宋体': 'STSong', 'simsun': 'SimSun', '宋体': 'SimSun', 'stfangsong': 'STFangsong', '华文仿宋': 'STFangsong',
            'stkaiti': 'STKaiti', '华文楷体': 'STKaiti', 'arial': 'Arial', 'calibri': 'Calibri', 'times new roman': 'Times New Roman',
            'courier new': 'Courier New'}


class _TplPkg:
    def __init__(self, path):
        import zipfile
        try: self.z = zipfile.ZipFile(path)
        except (zipfile.BadZipFile, FileNotFoundError, IsADirectoryError) as e:
            sys.stderr.write(f'not a pptx package ({e}). An old .ppt has to be converted to .pptx first.\n'); sys.exit(2)
        self.names = set(self.z.namelist())
        if 'ppt/presentation.xml' not in self.names:
            sys.stderr.write('no ppt/presentation.xml inside -- not a PowerPoint file\n'); sys.exit(2)
        self._x = {}

    def xml(self, name):
        from xml.etree import ElementTree as ET
        if name not in self._x: self._x[name] = ET.fromstring(self.z.read(name)) if name in self.names else None
        return self._x[name]

    def raw(self, name):
        return self.z.read(name).decode('utf-8', 'replace') if name in self.names else ''

    def rels(self, name):
        """{rId: (type_tail, target_part)} for one part."""
        import posixpath
        d, b = posixpath.split(name); root = self.xml(posixpath.join(d, '_rels', b + '.rels')); out = {}
        for r in (root.findall('rel:Relationship', _TNS) if root is not None else []):
            if r.get('TargetMode') == 'External': continue
            out[r.get('Id')] = (r.get('Type', '').rsplit('/', 1)[-1], posixpath.normpath(posixpath.join(d, r.get('Target'))))
        return out

    def first_rel(self, name, kind):
        return next((t for k, t in self.rels(name).values() if k == kind), None)


def _tpl_chroma(h):
    _, s, l = _rgb_to_hsl(h); return s >= 0.25 and 0.08 < l < 0.92

def _tpl_color(el, scheme):
    """(hex, alpha 0-1) of a fill-like element: srgbClr / schemeClr (through the theme) / sysClr / prstClr,
    with lumMod / lumOff / tint / shade / alpha applied. None when there is no colour."""
    if el is None: return None
    for tag in ('srgbClr', 'schemeClr', 'sysClr', 'prstClr'):
        c = el.find('a:' + tag, _TNS)
        if c is not None: break
    else: return None
    if tag == 'srgbClr': h = (c.get('val') or '').upper()
    elif tag == 'schemeClr': v = c.get('val'); h = scheme.get(_T_ALIAS.get(v, v), '')
    elif tag == 'sysClr': h = (c.get('lastClr') or '').upper()
    else: h = {'black': '000000', 'white': 'FFFFFF', 'red': 'FF0000', 'blue': '0000FF', 'green': '008000'}.get(c.get('val'), '')
    if not re.fullmatch(r'[0-9A-F]{6}', h): return None
    alpha = 1.0; hu, s, l = _rgb_to_hsl(h); mod = False
    for m in c:
        k = m.tag.split('}')[1]; v = int(m.get('val', '100000')) / 100000
        if k == 'lumMod': l *= v; mod = True
        elif k == 'lumOff': l += v; mod = True
        elif k == 'tint': l = l + (1 - l) * (1 - v); mod = True
        elif k == 'shade': l *= v; mod = True
        elif k == 'alpha': alpha = v
    if mod: h = _hsl_to_rgb(hu, s, max(0.0, min(1.0, l)))
    return h, alpha




def _tpl_xfrm(el):
    x = el.find('.//a:xfrm', _TNS)
    if x is None: return None
    off, ext = x.find('a:off', _TNS), x.find('a:ext', _TNS)
    try: return int(off.get('x')), int(off.get('y')), int(ext.get('cx')), int(ext.get('cy'))
    except (AttributeError, TypeError, ValueError): return None


# ---- raster: PNG in / out, scanline polygon fill (all stdlib) -----------------------------------
def _png_decode(b):
    """(w, h, RGBA bytearray) for an 8-bit non-interlaced PNG (grey / RGB / palette / grey+alpha / RGBA), else None."""
    import zlib
    if b[:8] != b'\x89PNG\r\n\x1a\n': return None
    import struct
    i, idat, plte, trns, w, h, depth, ctype, inter = 8, [], None, None, 0, 0, 0, 0, 0
    while i + 8 <= len(b):
        n = int.from_bytes(b[i:i + 4], 'big'); t = b[i + 4:i + 8]; d = b[i + 8:i + 8 + n]; i += 12 + n
        if t == b'IHDR': w, h = struct.unpack('>II', d[:8]); depth, ctype, inter = d[8], d[9], d[12]
        elif t == b'PLTE': plte = d
        elif t == b'tRNS': trns = d
        elif t == b'IDAT': idat.append(d)
        elif t == b'IEND': break
    ch = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(ctype)
    if not w or depth != 8 or not ch or (ctype == 3 and not plte): return None
    raw = zlib.decompress(b''.join(idat)); pos = 0
    out = bytearray(w * h * 4)

    def unfilter(pw, ph):
        """Undo the per-scanline filter for one pass, pw pixels wide and ph rows tall."""
        nonlocal pos
        stride = pw * ch; prev = bytearray(stride)
        for _y in range(ph):
            f = raw[pos]; row = bytearray(raw[pos + 1:pos + 1 + stride]); pos += 1 + stride
            if f == 1:
                for x in range(ch, stride): row[x] = (row[x] + row[x - ch]) & 255
            elif f == 2: row = bytearray((p + q) & 255 for p, q in zip(row, prev))
            elif f == 3:
                for x in range(stride): row[x] = (row[x] + ((row[x - ch] if x >= ch else 0) + prev[x]) // 2) & 255
            elif f == 4:
                for x in range(stride):
                    l_ = row[x - ch] if x >= ch else 0; c_ = prev[x - ch] if x >= ch else 0; u = prev[x]
                    p = l_ + u - c_; pa, pb, pc = abs(p - l_), abs(p - u), abs(p - c_)
                    row[x] = (row[x] + (l_ if pa <= pb and pa <= pc else u if pb <= pc else c_)) & 255
            prev = row; yield row

    def px4(row, x):
        if ctype == 6: return row[4 * x:4 * x + 4]
        if ctype == 2: return bytes((row[3 * x], row[3 * x + 1], row[3 * x + 2], 255))
        if ctype == 3:
            k = row[x]
            if 3 * k + 2 >= len(plte): return b'\0\0\0\0'
            return bytes((plte[3 * k], plte[3 * k + 1], plte[3 * k + 2], trns[k] if trns and k < len(trns) else 255))
        if ctype == 0: return bytes((row[x], row[x], row[x], 255))
        return bytes((row[2 * x], row[2 * x], row[2 * x], row[2 * x + 1]))

    if inter:
        # Adam7: the image arrives as seven passes of scattered pixels. Icon-sized art inside PPT templates is
        # routinely saved interlaced, and refusing it meant the decoration was dropped on every machine without
        # Pillow or sips -- which is the normal case in the agent's sandbox.
        for xs, ys, xst, yst in ((0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)):
            pw, ph = max(0, -(-(w - xs) // xst)), max(0, -(-(h - ys) // yst))
            if not pw or not ph: continue
            for y, row in enumerate(unfilter(pw, ph)):
                o = (ys + y * yst) * w * 4
                for x in range(pw):
                    q = o + (xs + x * xst) * 4; out[q:q + 4] = px4(row, x)
        return w, h, out
    for y, row in enumerate(unfilter(w, h)):
        o = y * w * 4
        if ctype == 6: out[o:o + w * 4] = row
        elif ctype == 2:
            for x in range(w): out[o + 4 * x:o + 4 * x + 4] = bytes((row[3 * x], row[3 * x + 1], row[3 * x + 2], 255))
        elif ctype == 3:
            for x in range(w):
                k = row[x]; a = trns[k] if trns and k < len(trns) else 255
                out[o + 4 * x:o + 4 * x + 4] = bytes((plte[3 * k], plte[3 * k + 1], plte[3 * k + 2], a))
        elif ctype == 0:
            for x in range(w): out[o + 4 * x:o + 4 * x + 4] = bytes((row[x], row[x], row[x], 255))
        else:
            for x in range(w): out[o + 4 * x:o + 4 * x + 4] = bytes((row[2 * x], row[2 * x], row[2 * x], row[2 * x + 1]))
    return w, h, out

def _png_encode(w, h, px):
    import zlib, struct
    stride = w * 4; raw = bytearray()
    for y in range(h): raw.append(0); raw += px[y * stride:(y + 1) * stride]
    def chunk(t, d): return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(bytes(raw), 6)) + chunk(b'IEND', b''))

class _Canvas:
    """A transparent RGBA bitmap the band furniture is painted into. Only two operations are needed now that the
    template's body shapes are no longer traced: fill a polygon (the EMF art and the thin rules) and composite a
    picture (logos, page backgrounds)."""

    def __init__(s, w, h): s.w, s.h, s.px, s.used = w, h, bytearray(w * h * 4), False

    def fill(s, polys, rgb, alpha=1.0, evenodd=False):
        edges = []
        for pts in polys:
            for i in range(len(pts)):
                (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % len(pts)]
                if y0 == y1: continue
                d = 1
                if y0 > y1: x0, y0, x1, y1, d = x1, y1, x0, y0, -1
                edges.append((y0, y1, x0, (x1 - x0) / (y1 - y0), d))
        if not edges: return
        r, g, b = (int(rgb[i:i + 2], 16) for i in (0, 2, 4)); a = max(0, min(255, round(alpha * 255)))
        if a == 0: return
        col = bytes((r, g, b, a)); s.used = True
        y0 = max(0, int(min(e[0] for e in edges))); y1 = min(s.h - 1, int(max(e[1] for e in edges)) + 1)
        for y in range(y0, y1 + 1):
            yc = y + 0.5; xs = sorted((ex + (yc - ey0) * k, d) for ey0, ey1, ex, k, d in edges if ey0 <= yc < ey1)
            if not xs: continue
            spans = []
            if evenodd: spans = [(xs[i][0], xs[i + 1][0]) for i in range(0, len(xs) - 1, 2)]
            else:
                wnd = 0; start = 0.0
                for x, d in xs:
                    prev = wnd; wnd += d
                    if prev == 0 and wnd != 0: start = x
                    elif prev != 0 and wnd == 0: spans.append((start, x))
            row = y * s.w * 4
            for xa, xb in spans:
                ia, ib = max(0, int(round(xa))), min(s.w, int(round(xb)))
                if ib <= ia: continue
                if a == 255: s.px[row + ia * 4:row + ib * 4] = col * (ib - ia)
                else:
                    for x in range(ia, ib): s._over(row + 4 * x, r, g, b, a)

    def _over(s, o, r, g, b, a):
        da = s.px[o + 3]
        if da == 0: s.px[o:o + 4] = bytes((r, g, b, a)); return
        oa = a + da * (255 - a) // 255
        if oa == 0: return
        for k, v in enumerate((r, g, b)): s.px[o + k] = (v * a + s.px[o + k] * da * (255 - a) // 255) // oa
        s.px[o + 3] = oa

    def stroke(s, pts, closed, width, rgb, alpha=1.0):
        import math
        hw = max(0.5, width / 2); quads = []
        seq = pts + [pts[0]] if closed else pts
        for (x0, y0), (x1, y1) in zip(seq, seq[1:]):
            dx, dy = x1 - x0, y1 - y0; L = math.hypot(dx, dy)
            if L == 0: continue
            nx, ny = -dy / L * hw, dx / L * hw
            quads.append([(x0 + nx, y0 + ny), (x1 + nx, y1 + ny), (x1 - nx, y1 - ny), (x0 - nx, y0 - ny)])
        for q in quads: s.fill([q], rgb, alpha)

    def blit(s, w, h, src, x, y, dw, dh):
        """Composite an RGBA picture (w x h) onto the canvas box (x, y, dw, dh), nearest neighbour, source over."""
        x, y, dw, dh = int(round(x)), int(round(y)), max(1, int(round(dw))), max(1, int(round(dh)))
        for ty in range(max(0, y), min(s.h, y + dh)):
            sy = min(h - 1, (ty - y) * h // dh); row = ty * s.w * 4
            for tx in range(max(0, x), min(s.w, x + dw)):
                sx = min(w - 1, (tx - x) * w // dw); o = (sy * w + sx) * 4; a = src[o + 3]
                if a == 0: continue
                s.used = True
                if a == 255: s.px[row + 4 * tx:row + 4 * tx + 4] = src[o:o + 4]
                else: s._over(row + 4 * tx, src[o], src[o + 1], src[o + 2], a)




def _bezier(p0, p1, p2, p3, n=16):
    out = []
    for i in range(1, n + 1):
        t = i / n; u = 1 - t
        out.append((u ** 3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t ** 3 * p3[0],
                    u ** 3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t ** 3 * p3[1]))
    return out


# ---- EMF (classic GDI records) -> drawing ops ------------------------------------------------------
def _emf_ops(b):
    """Parse a GDI EMF into ([(kind, polys, rgb, alpha, evenodd, width)], frame=(x0, y0, x1, y1) in device units).
    kind 'fill' or 'stroke'. Clipping, text and embedded bitmaps are ignored. None if this is not an EMF."""
    import struct
    if len(b) < 88 or struct.unpack('<I', b[:4])[0] != 1 or b[40:44] != b' EMF': return None
    bounds = struct.unpack('<4i', b[8:24]); frame = struct.unpack('<4i', b[24:40])
    devw, devh, mmw, mmh = struct.unpack('<4i', b[72:88])
    if mmw > 0 and mmh > 0 and devw > 0 and devh > 0:
        kx, ky = devw / (mmw * 100), devh / (mmh * 100)
        fr = (frame[0] * kx, frame[1] * ky, frame[2] * kx, frame[3] * ky)
    else: fr = tuple(float(v) for v in bounds)
    if fr[2] - fr[0] < 1 or fr[3] - fr[1] < 1: fr = tuple(float(v) for v in bounds)
    st = {'wo': (0, 0), 'we': None, 'vo': (0, 0), 've': None, 'm': (1, 0, 0, 1, 0, 0), 'brush': ('FFFFFF', 1.0),
          'pen': ('000000', 1.0), 'mode': 1}
    stock = {0: ('brush', ('FFFFFF', 1.0)), 1: ('brush', ('C0C0C0', 1.0)), 2: ('brush', ('808080', 1.0)),
             3: ('brush', ('404040', 1.0)), 4: ('brush', ('000000', 1.0)), 5: ('brush', None),
             6: ('pen', ('FFFFFF', 1.0)), 7: ('pen', ('000000', 1.0)), 8: ('pen', None)}
    objs, saved, ops = {}, [], []
    figs, cur, inpath, pos = [], [], False, (0, 0)
    def ref(c): return '%02X%02X%02X' % (c & 255, (c >> 8) & 255, (c >> 16) & 255)
    def T(x, y):
        a, bb, c, d, e, f = st['m']; xw, yw = x * a + y * c + e, x * bb + y * d + f
        if st['we'] and st['ve'] and st['we'][0] and st['we'][1]:
            return ((xw - st['wo'][0]) * st['ve'][0] / st['we'][0] + st['vo'][0], (yw - st['wo'][1]) * st['ve'][1] / st['we'][1] + st['vo'][1])
        return (xw - st['wo'][0] + st['vo'][0], yw - st['wo'][1] + st['vo'][1])
    def penw():
        w = st['pen'][2] if st['pen'] and len(st['pen']) > 2 else 1
        if st['we'] and st['ve'] and st['we'][0]: w = w * abs(st['ve'][0] / st['we'][0])
        return max(1.0, w)
    def emit(polys, fill, stroke, closed=True):
        polys = [p for p in polys if len(p) >= 2]
        if not polys: return
        if fill and st['brush']: ops.append(('fill', [p for p in polys if len(p) >= 3], st['brush'][0], st['brush'][1], st['mode'] == 1, 0))
        if stroke and st['pen']: ops.append(('stroke', polys, st['pen'][0], st['pen'][1], closed, penw()))
    def pts16(d, n, off): return [T(*struct.unpack('<hh', d[off + 4 * i:off + 4 * i + 4])) for i in range(n)]
    def pts32(d, n, off): return [T(*struct.unpack('<ii', d[off + 8 * i:off + 8 * i + 8])) for i in range(n)]
    i = 0
    while i + 8 <= len(b):
        t, n = struct.unpack('<II', b[i:i + 8]); d = b[i + 8:i + n]
        if n < 8: break
        try:
            if t == 14: break
            elif t == 9: st['we'] = struct.unpack('<ii', d[:8])
            elif t == 10: st['wo'] = struct.unpack('<ii', d[:8])
            elif t == 11: st['ve'] = struct.unpack('<ii', d[:8])
            elif t == 12: st['vo'] = struct.unpack('<ii', d[:8])
            elif t == 19: st['mode'] = struct.unpack('<I', d[:4])[0]
            elif t == 35: st['m'] = struct.unpack('<6f', d[:24])
            elif t == 36:
                m = struct.unpack('<6f', d[:24]); mode = struct.unpack('<I', d[24:28])[0]
                def mul(p, q): return (p[0] * q[0] + p[1] * q[2], p[0] * q[1] + p[1] * q[3], p[2] * q[0] + p[3] * q[2],
                                       p[2] * q[1] + p[3] * q[3], p[4] * q[0] + p[5] * q[2] + q[4], p[4] * q[1] + p[5] * q[3] + q[5])
                st['m'] = (1, 0, 0, 1, 0, 0) if mode == 1 else mul(m, st['m']) if mode == 2 else mul(st['m'], m) if mode == 3 else m
            elif t == 33: saved.append(dict(st))
            elif t == 34:
                k = struct.unpack('<i', d[:4])[0]
                for _ in range(min(len(saved), abs(k) if k < 0 else 1)): st.update(saved.pop())
            elif t == 39:
                ih, style, col = struct.unpack('<III', d[:12]); objs[ih] = ('brush', None if style == 1 else (ref(col), 1.0))
            elif t == 38:
                ih, style, w, _h, col = struct.unpack('<IIiiI', d[:20]); objs[ih] = ('pen', None if style & 15 == 5 else (ref(col), 1.0, w))
            elif t == 95:
                ih = struct.unpack('<I', d[:4])[0]; style, w, _bs, col = struct.unpack('<IIII', d[20:36])
                objs[ih] = ('pen', None if style & 15 == 5 else (ref(col), 1.0, w))
            elif t == 37:
                ih = struct.unpack('<I', d[:4])[0]
                o = stock.get(ih & 0x7FFFFFFF) if ih & 0x80000000 else objs.get(ih)
                if o: st[o[0]] = o[1]
            elif t == 40: objs.pop(struct.unpack('<I', d[:4])[0], None)
            elif t == 59: inpath, figs, cur = True, [], []
            elif t == 60: inpath = False; figs.append(cur) if cur else None; cur = []
            elif t == 68: inpath, figs, cur = False, [], []
            elif t == 27:
                pos = T(*struct.unpack('<ii', d[:8]))
                if inpath:
                    if len(cur) > 1: figs.append(cur)
                    cur = [pos]
            elif t == 54:
                p = T(*struct.unpack('<ii', d[:8]))
                if inpath: cur = cur or [pos]; cur.append(p)
                else: emit([[pos, p]], False, True, False)
                pos = p
            elif t == 61:
                if inpath and cur: figs.append(cur); cur = []
            elif t in (88, 89, 5, 6):
                cnt = struct.unpack('<I', d[16:20])[0]; ps = pts16(d, cnt, 20) if t in (88, 89) else pts32(d, cnt, 20)
                if t in (89, 6): seg = ps
                else:
                    seg, p0 = [], pos
                    for k in range(0, len(ps) - 2, 3): seg += _bezier(p0, ps[k], ps[k + 1], ps[k + 2]); p0 = ps[k + 2]
                if inpath: cur = cur or [pos]; cur += seg
                else: emit([[pos] + seg], False, True, False)
                pos = ps[-1] if ps else pos
            elif t in (85, 86, 87, 2, 3, 4):
                cnt = struct.unpack('<I', d[16:20])[0]; ps = pts16(d, cnt, 20) if t in (85, 86, 87) else pts32(d, cnt, 20)
                if t in (85, 2):
                    seg = [ps[0]] if ps else []
                    for k in range(1, len(ps) - 2, 3): seg += _bezier(seg[-1], ps[k], ps[k + 1], ps[k + 2])
                    ps = seg
                if inpath:
                    if len(cur) > 1: figs.append(cur)
                    figs.append(ps); cur = []
                else: emit([ps], t in (86, 3), True, t in (86, 3))
            elif t in (91, 90, 8, 7):
                npoly, _tot = struct.unpack('<II', d[16:24]); counts = struct.unpack('<%dI' % npoly, d[24:24 + 4 * npoly])
                off = 24 + 4 * npoly; polys = []
                for c in counts:
                    polys.append(pts16(d, c, off) if t in (91, 90) else pts32(d, c, off)); off += c * (4 if t in (91, 90) else 8)
                if inpath: figs += polys
                else: emit(polys, t in (91, 8), True, t in (91, 8))
            elif t in (43, 42):
                l_, tp, r_, bt = struct.unpack('<4i', d[:16])
                if t == 43: ps = [T(l_, tp), T(r_, tp), T(r_, bt), T(l_, bt)]
                else:
                    import math
                    cx, cy, rx, ry = (l_ + r_) / 2, (tp + bt) / 2, (r_ - l_) / 2, (bt - tp) / 2
                    ps = [T(cx + rx * math.cos(k * math.pi / 32), cy + ry * math.sin(k * math.pi / 32)) for k in range(64)]
                emit([ps], True, True)
            elif t in (62, 63, 64):
                polys = figs + ([cur] if len(cur) > 1 else [])
                emit(polys, t in (62, 63), t in (63, 64), True); figs, cur = [], []
            elif t == 67: figs, cur = [], []          # SELECTCLIPPATH consumes the path; clipping itself is ignored
        except struct.error:
            pass
        i += n
    return ops, fr


# ---- preset shapes -> polygon (shape-local coordinates, w x h) --------------------------------------




_ROLE_RX = [('cover', re.compile(r'封面|cover|title slide|标题幻灯片|^标题$', re.I)),
            ('end', re.compile(r'结束|结尾|封底|致谢|谢谢|感谢|thank|closing|ending|^end', re.I)),
            ('toc', re.compile(r'目录|table of contents|\bcontents\b|\bagenda\b|议程|catalog', re.I)),
            ('section', re.compile(r'扉页|章节|节标题|过渡|section|divider|chapter|part\b|transition', re.I)),
            ('content', re.compile(r'内容|正文|content|title and|标题和|仅标题|title only|text|body', re.I))]
_SKIP_PH = ('dt', 'ftr', 'hdr', 'sldImg', 'clipArt', 'chart', 'tbl', 'dgm', 'media', 'pic', 'obj')


class _Tpl:
    def __init__(s, path):
        import os
        s.path = path; s.pkg = pkg = _TplPkg(path); pres = pkg.xml('ppt/presentation.xml'); sz = pres.find('p:sldSz', _TNS)
        s.W, s.H = (int(sz.get('cx')), int(sz.get('cy'))) if sz is not None else (12192000, 6858000)
        s.kx, s.ky = 1280 / s.W, 720 / s.H; s.pt = 720 / (s.H / 12700)          # px per EMU (x, y) and px per point
        r = s.W / s.H
        s.ratio = '16:9' if abs(r - 16 / 9) < 0.03 else '4:3' if abs(r - 4 / 3) < 0.03 else '16:10' if abs(r - 1.6) < 0.03 else f'{r:.2f}:1'
        prels = pkg.rels('ppt/presentation.xml')
        s.slides = [prels[x.get(_T_RID)][1] for x in pres.findall('p:sldIdLst/p:sldId', _TNS) if x.get(_T_RID) in prels]
        s.masters = [prels[m.get(_T_RID)][1] for m in pres.findall('p:sldMasterIdLst/p:sldMasterId', _TNS) if m.get(_T_RID) in prels] \
            or sorted(n for n in pkg.names if re.match(r'ppt/slideMasters/slideMaster\d+\.xml$', n))
        s.layouts = sorted((n for n in pkg.names if re.match(r'ppt/slideLayouts/slideLayout\d+\.xml$', n)),
                           key=lambda n: int(re.findall(r'\d+', n)[-1]))
        s.layout_of = {x: pkg.first_rel(x, 'slideLayout') for x in s.slides}
        s.master_of = {l: pkg.first_rel(l, 'slideMaster') or (s.masters[0] if s.masters else None) for l in s.layouts}
        s.scheme, s.tfonts = {}, {}
        troot = pkg.xml((pkg.first_rel(s.masters[0], 'theme') if s.masters else None) or 'ppt/theme/theme1.xml')
        if troot is not None:
            cs = troot.find('.//a:clrScheme', _TNS)
            for el in (cs if cs is not None else []):
                c = el.find('a:srgbClr', _TNS); x = el.find('a:sysClr', _TNS)
                v = c.get('val') if c is not None else x.get('lastClr') if x is not None else None
                if v: s.scheme[el.tag.split('}')[1]] = v.upper()
            for kind in ('major', 'minor'):
                f = troot.find(f'.//a:{kind}Font', _TNS)
                if f is None: continue
                for script in ('latin', 'ea'):
                    e = f.find(f'a:{script}', _TNS)
                    if e is not None and e.get('typeface'): s.tfonts[f'{kind}_{script}'] = e.get('typeface')
                hans = f.find("a:font[@script='Hans']", _TNS)
                if not s.tfonts.get(f'{kind}_ea') and hans is not None and hans.get('typeface'): s.tfonts[f'{kind}_ea'] = hans.get('typeface')
        s.notes = []

    def name(s, part):
        root = s.pkg.xml(part); c = root.find('p:cSld', _TNS) if root is not None else None
        return (c.get('name') if c is not None else '') or ''

    # --- colours / fonts actually used
    def painted(s):
        import collections
        counter = collections.Counter()
        used = sorted({l for l in s.layout_of.values() if l}) or s.layouts
        for part in s.slides + used + s.masters:
            src = re.sub(r'<p:style>.*?</p:style>', '', s.pkg.raw(part), flags=re.S)
            for v in re.findall(r'<a:srgbClr val="([0-9A-Fa-f]{6})"', src): counter[v.upper()] += 1
            for v in re.findall(r'<a:schemeClr val="(\w+)"', src):
                h = s.scheme.get(_T_ALIAS.get(v, v))
                if h: counter[h] += 1
        return counter

    def font(s, tf):
        """A template typeface -> (whitelisted fontFamily, class)."""
        m = re.match(r'\+(mj|mn)-(lt|ea|cs)', tf or '')
        if m: tf = s.tfonts.get(('major' if m.group(1) == 'mj' else 'minor') + '_' + ('latin' if m.group(2) == 'lt' else 'ea'), '')
        tf = (tf or '').strip()
        cls = 'mono' if _T_MONO.search(tf) else 'serif' if _T_SERIF.search(tf) else 'sans'
        hit = _T_FONTS.get(tf.lower())
        if hit: return hit, cls
        if not tf: return 'Microsoft YaHei', 'sans'
        return {'serif': 'STSong', 'mono': 'Courier New'}.get(cls, 'Microsoft YaHei'), cls

    # --- geometry
    def box(s, g, small_keep_ratio=False):
        """EMU (x, y, w, h) -> canvas px. A 4:3 template is stretched to 16:9 horizontally; small art keeps its ratio
        (left-anchored on the left half, right-anchored on the right half) so logos and badges are not squashed."""
        x, y, w, h = g
        X, Y, Wd, Hd = x * s.kx, y * s.ky, w * s.kx, h * s.ky
        if small_keep_ratio and abs(s.kx - s.ky) > 1e-9 and w * s.ky < 0.25 * 1280:
            nw = w * s.ky
            X = X if (x + w / 2) < s.W / 2 else (x + w) * s.kx - nw
            Wd = nw
        return X, Y, Wd, Hd

    def elements(s, part, ox=0.0, oy=0.0, sx=1.0, sy=1.0, tree=None):
        """Flatten a part's shape tree into (kind, el, emu_box, rot, flipH, flipV) with group transforms applied."""
        root = s.pkg.xml(part)
        if tree is None:
            tree = root.find('p:cSld/p:spTree', _TNS) if root is not None else None
            if tree is None: return []
        out = []
        for el in tree:
            tag = el.tag.split('}')[1]
            if tag == 'grpSp':
                x = el.find('p:grpSpPr/a:xfrm', _TNS)
                try:
                    off, ext = x.find('a:off', _TNS), x.find('a:ext', _TNS); co, ce = x.find('a:chOff', _TNS), x.find('a:chExt', _TNS)
                    gx, gy, gw, gh = int(off.get('x')), int(off.get('y')), int(ext.get('cx')), int(ext.get('cy'))
                    cx, cy, cw, chh = int(co.get('x')), int(co.get('y')), int(ce.get('cx')) or 1, int(ce.get('cy')) or 1
                    nsx, nsy = sx * gw / cw, sy * gh / chh
                    out += s.elements(part, ox + (gx - cx * gw / cw) * sx, oy + (gy - cy * gh / chh) * sy, nsx, nsy, el)
                except (AttributeError, TypeError, ValueError):
                    s.notes.append(f'{part}: a group without a usable transform was skipped')
                continue
            if tag not in ('sp', 'pic', 'cxnSp'): continue
            xf = el.find('p:spPr/a:xfrm', _TNS) if tag != 'pic' else el.find('p:spPr/a:xfrm', _TNS)
            g = None
            if xf is not None and xf.find('a:off', _TNS) is not None:
                o, e = xf.find('a:off', _TNS), xf.find('a:ext', _TNS)
                g = (ox + int(o.get('x')) * sx, oy + int(o.get('y')) * sy, int(e.get('cx')) * sx, int(e.get('cy')) * sy)
            rot = int(xf.get('rot', '0')) / 60000 if xf is not None else 0.0
            fh = xf is not None and xf.get('flipH') == '1'; fv = xf is not None and xf.get('flipV') == '1'
            out.append((tag, el, g, rot, fh, fv))
        return out

    def ph(s, el):
        p = el.find('p:nvSpPr/p:nvPr/p:ph', _TNS)
        if p is None: return None
        return (p.get('type') or 'body'), p.get('idx')

    def inherited_ph(s, master, phinfo):
        """The master placeholder a layout placeholder inherits from (same type, else same idx, else body)."""
        typ, idx = phinfo; best = None
        for kind, el, g, *_ in s.elements(master):
            p = s.ph(el) if kind == 'sp' else None
            if not p: continue
            t2 = p[0]
            if t2 == typ or (typ in ('ctrTitle',) and t2 == 'title') or (typ == 'subTitle' and t2 == 'body'):
                return el, g
            if best is None and t2 == 'body': best = (el, g)
        return best or (None, None)

    def text_style(s, chain, master, kind):
        """Font size px, colour, bold, family, class, align, anchor, line spacing and insets for a placeholder,
        resolved along layout ph (list style, then the prompt text's own run) -> master ph -> master txStyles."""
        A = lambda tag: '{%s}%s' % (_TNS['a'], tag)
        mroot = s.pkg.xml(master) if master else None
        sty = None
        if mroot is not None:
            key = 'titleStyle' if kind in ('title', 'ctrTitle') else 'bodyStyle' if kind in ('body', 'subTitle', 'obj') else 'otherStyle'
            sty = mroot.find(f'p:txStyles/p:{key}', _TNS)
        paras, runs = [], []                            # (lvl1pPr-like, run-properties-like) in priority order
        for c in chain:
            if c is None: continue
            lv = c.find('.//a:lstStyle/a:lvl1pPr', _TNS); pp = c.find('.//a:p/a:pPr', _TNS)
            for e in (lv, pp):
                if e is not None: paras.append(e); d = e.find('a:defRPr', _TNS); runs.append(d) if d is not None else None
            for e in (c.find('.//a:r/a:rPr', _TNS), c.find('.//a:endParaRPr', _TNS)):
                if e is not None: runs.append(e)
        if sty is not None:
            lv = sty.find('a:lvl1pPr', _TNS)
            if lv is not None:
                paras.append(lv); d = lv.find('a:defRPr', _TNS); runs.append(d) if d is not None else None
        def first(items, fn):
            for e in items:
                v = fn(e)
                if v is not None: return v
            return None
        sz = first(runs, lambda e: e.get('sz'))
        col = first(runs, lambda e: _tpl_color(e.find('a:solidFill', _TNS), s.scheme) if e.find('a:solidFill', _TNS) is not None else None)
        b = first(runs, lambda e: e.get('b'))
        tf = first(runs, lambda e: next((x.get('typeface') for x in (e.find('a:ea', _TNS), e.find('a:latin', _TNS))
                                         if x is not None and x.get('typeface')), None))
        algn = first(paras, lambda e: e.get('algn'))
        lns = first(paras, lambda e: (lambda p: int(p.get('val')) / 100000 if p is not None and p.get('val') else None)(e.find('a:lnSpc/a:spcPct', _TNS)))
        bp = [c.find('.//a:bodyPr', _TNS) for c in chain if c is not None]; bp = [e for e in bp if e is not None]
        anchor = first(bp, lambda e: e.get('anchor'))
        ins = {k: first(bp, lambda e, k=k: e.get(k)) for k in ('lIns', 'tIns', 'rIns', 'bIns')}
        fam, cls = s.font(tf or ('+mj-ea' if kind in ('title', 'ctrTitle') else '+mn-ea'))
        return {'size': round((int(sz) / 100 if sz else (44 if kind in ('title', 'ctrTitle') else 18)) * s.pt),
                'color': (col or (s.scheme.get('dk1', '000000'), 1.0))[0], 'bold': b in ('1', 'true'), 'family': fam,
                'class': cls, 'align': {'ctr': 'center', 'r': 'right', 'just': 'left', 'dist': 'center'}.get(algn or 'l', 'left'),
                'anchor': {'ctr': 'center', 'b': 'flex-end'}.get(anchor or 't', 'flex-start'),
                'lh': max(0.9, min(1.5, lns or 1.0)),
                'ins': tuple(int(ins[k]) if ins[k] is not None else d for k, d in
                             (('lIns', 91440), ('tIns', 45720), ('rIns', 91440), ('bIns', 45720))),
                'sample': ''.join(t.text or '' for t in (chain[0].iter(A('t')) if chain[0] is not None else [])).strip(),
                'typeface': tf}

    def bg_solid(s, part):
        root = s.pkg.xml(part); bg = root.find('p:cSld/p:bg', _TNS) if root is not None else None
        if bg is None: return None
        pr = bg.find('p:bgPr', _TNS)
        if pr is not None:
            sf = pr.find('a:solidFill', _TNS)
            if sf is not None: c = _tpl_color(sf, s.scheme); return ('color', c[0]) if c else None
            gf = pr.find('a:gradFill', _TNS)
            if gf is not None:
                cs = [c[0] for c in (_tpl_color(g, s.scheme) for g in gf.findall('.//a:gs', _TNS)) if c]
                if cs: return ('color', cs[0])
            blip = pr.find('a:blipFill/a:blip', _TNS)
            if blip is not None:
                t = s.pkg.rels(part).get(blip.get(_T_EMBED)); return ('image', t[1]) if t else None
        ref = bg.find('p:bgRef', _TNS)
        if ref is not None: c = _tpl_color(ref, s.scheme); return ('color', c[0]) if c else None
        return None

    def slide_sig(s, sl):
        """(text shape count, biggest font pt, joined text) of one of the template's own slides -- used to tell
        a cover / section / toc / thanks page apart from a content page by what is actually on it."""
        n, big, txt = 0, 0, []
        for kind, el, g, *_ in s.elements(sl):
            if kind != 'sp': continue
            t = ''.join(x.text or '' for x in el.iter('{%s}t' % _TNS['a'])).strip()
            if not t: continue
            if s.ph(el) and s.ph(el)[0] in _SKIP_PH: continue
            n += 1; txt.append(t)
            for tag in ('rPr', 'defRPr'):
                for e in el.iter('{%s}%s' % (_TNS['a'], tag)):
                    if e.get('sz'): big = max(big, int(e.get('sz')) / 100)
        return n, big, ' '.join(txt)

    def rep_slides(s, uses):
        """One representative slide per page type, read off the template's own slides. A template whose design lives
        on the slides (text boxes, no layout placeholders -- the common case for Chinese templates) is only
        reproducible from these; the layouts are empty."""
        sig = {sl: s.slide_sig(sl) for sl in s.slides}
        out = {}
        if s.slides: out['cover'] = s.slides[0]
        for sl in s.slides:
            if 'toc' not in out and _ROLE_RX[2][1].search(sig[sl][2]): out['toc'] = sl
        sect = [sl for sl in s.slides[1:] if sig[sl][0] <= 6 and
                (re.search(r'\bPART\b|\bCHAPTER\b|章节', sig[sl][2], re.I) or
                 (sig[sl][1] >= 40 and re.search(r'(?<!\d)\d{1,2}(?!\d)', sig[sl][2]))) and sl is not out.get('toc')]
        if sect: out['section'] = sect[0]
        last = s.slides[-1] if len(s.slides) > 1 else None
        if last is not None and last not in out.values() and (sig[last][0] <= 4 or
                re.search(r'thank|谢|感谢|结束', sig[last][2], re.I)): out['end'] = last
        body = [sl for sl in s.slides if sl not in out.values()]
        if body: out['content'] = max(body, key=lambda sl: sig[sl][0])
        return out

    def roles(s):
        """Pick one layout per page type: the layout the template's own slides use for it, else the first name match."""
        named = {}
        for l in s.layouts:
            nm = s.name(l)
            for role, rx in _ROLE_RX:
                if rx.search(nm): named.setdefault(role, []).append(l); break
        uses = {}
        for i, x in enumerate(s.slides):
            l = s.layout_of.get(x)
            if l: uses.setdefault(l, []).append(i)
        out = {}
        for role in ('cover', 'toc', 'section', 'end'):
            cands = named.get(role, [])
            used = [l for l in cands if l in uses]
            if used: out[role] = min(used, key=lambda l: uses[l][0])
            elif cands: out[role] = cands[0]
        if 'cover' not in out and s.slides and s.layout_of.get(s.slides[0]): out['cover'] = s.layout_of[s.slides[0]]
        taken = set(out.values())
        def busy(l):                                     # share of the canvas taken by pictures / picture placeholders
            a = 0.0
            for k, el, g, *_ in s.elements(l):
                p = s.ph(el) if k == 'sp' else None
                if g and (k == 'pic' or (p and p[0] in ('pic', 'clipArt', 'media'))): a += g[2] * g[3] / (s.W * s.H)
            return a
        pool = [l for l in s.layouts if l not in taken and s.has_title(l)] or [l for l in s.layouts if l not in taken]
        avoid = re.compile(r'图片|图像|插图|照片|picture|photo|image|quote|引用', re.I)
        plain = re.compile(r'仅标题|只有标题|title only|标题和内容|title and content|标题与内容', re.I)
        # the layout the template's own content slides use most is what "looks like the template"
        pool = [l for l in s.layouts if l not in taken and (s.has_title(l) or uses.get(l))] or [l for l in s.layouts if l not in taken]
        content = sorted(pool, key=lambda l: (busy(l) > 0.15, bool(avoid.search(s.name(l))), -len(uses.get(l, [])),
                                              not plain.search(s.name(l)), not _ROLE_RX[4][1].search(s.name(l)), s.layouts.index(l)))
        if content: out['content'] = content[0]
        elif s.layouts: out['content'] = s.layouts[0]
        reps = s.rep_slides(uses)                        # the template's own slide for each page type
        for role, sl in reps.items():
            l = s.layout_of.get(sl)
            if l and (role not in out or not s.has_title(out[role])): out[role] = l
        for role in reps:
            if role not in out and s.layouts: out[role] = s.layout_of.get(reps[role]) or s.layouts[0]
        return out, uses, reps

    def has_title(s, layout):
        return any(s.ph(el) and s.ph(el)[0] in ('title', 'body', 'ctrTitle') for k, el, *_ in s.elements(layout) if k == 'sp')


def _tpl_shrink(dec, max_side):
    """Nearest-neighbour downscale of a decoded (w, h, RGBA) picture. Good enough: the result is either composited
    at 1/8 scale to sample a colour, or drawn small into the decoration layer."""
    w, h, px = dec
    f = max(w, h) / float(max_side)
    if f <= 1.0: return dec
    nw, nh = max(1, int(w / f)), max(1, int(h / f))
    out = bytearray(nw * nh * 4)
    for y in range(nh):
        src, dst = min(h - 1, int(y * f)) * w * 4, y * nw * 4
        for x in range(nw):
            s = src + min(w - 1, int(x * f)) * 4
            out[dst:dst + 4] = px[s:s + 4]; dst += 4
    return nw, nh, out


def _raster(data, ext, max_side=None):
    """Decode a picture to (w, h, RGBA). PNG natively; others via Pillow or sips."""
    dims = _dims_from_bytes(data)
    if ext == 'png' and dims and dims[0] * dims[1] <= 16_000_000:
        # Template decorations are routinely high-res art placed small -- a 1600px glow used as a 120px corner
        # piece. Handing those to Pillow / sips just because they need shrinking loses them outright wherever
        # neither exists, which is the common case; decode here and shrink instead.
        dec = _png_decode(data)
        if dec and max_side and max(dec[0], dec[1]) > max_side * 1.5: dec = _tpl_shrink(dec, max_side)
        if dec: return dec
    try:
        import io
        from PIL import Image
        im = Image.open(io.BytesIO(data)).convert('RGBA')
        if max_side: im.thumbnail((max_side, max_side))
        return im.size[0], im.size[1], bytearray(im.tobytes())
    except Exception: pass
    import shutil, subprocess, tempfile, os
    if not shutil.which('sips'): return None
    with tempfile.TemporaryDirectory() as d:
        src, dst = os.path.join(d, 'in.' + ext), os.path.join(d, 'out.png'); open(src, 'wb').write(data)
        cmd = ['sips', '-s', 'format', 'png'] + (['-Z', str(int(max_side))] if max_side else []) + [src, '--out', dst]
        r = subprocess.run(cmd, capture_output=True)
        return _png_decode(open(dst, 'rb').read()) if r.returncode == 0 and os.path.exists(dst) else None


_BAND_TOP, _BAND_BOT = 0.22, 0.9     # header / footer bands: what a template parks there is page furniture
_BAND_BAR = 48.0                     # px: a filled shape thinner than this, inside a band, is a rule -- not a panel
_BAND_PIC = 0.15                     # share of the page above which a picture in a band is not furniture either


def _tpl_page(tpl, role, layout, assets, uses, rep_slide=None):
    """One page type of the template -> a shell holding only what cannot be designed from scratch: the page
    background, and the art the template keeps in its header / footer bands (rules, logos, page furniture).

    Shapes in the body are deliberately NOT traced. Tracing them is what produced the whole bug trail: bounding-box
    squares for every preset the rasteriser did not know, flat slabs for gradients, cut-out photos that lost their
    outline, cards without their shadow, and a cover whose small picture hung in mid-air once the bar behind it was
    dropped. The deck's own layout is designed from the template's palette and type instead, which is also what the
    template's own identity actually consists of once its background art is in place.

    Returns dict(jsx, notes, files, area, bg, layout, layout_name, dropped)."""
    import os
    master = tpl.master_of.get(layout) or (tpl.masters[0] if tpl.masters else None)
    lroot = tpl.pkg.xml(layout)
    show_master = lroot is None or lroot.get('showMasterSp', '1') not in ('0', 'false')
    sample = [tpl.slides[i] for i in uses.get(layout, [])]
    rep = rep_slide if rep_slide is not None else (sample[0] if sample else None)
    # OOXML resolves a page's background slide -> layout -> master. Reading only the last two loses every template
    # that sets its art on the slide itself (a 'blank' layout with <p:bg> on slide1).
    bg = (tpl.bg_solid(rep) if rep is not None else None) or tpl.bg_solid(layout) \
        or (tpl.bg_solid(master) if master else None) or ('color', tpl.scheme.get('lt1', 'FFFFFF'))
    base = bg[1] if bg[0] == 'color' and bg[1] else tpl.scheme.get('lt1', 'FFFFFF')
    S = 2
    art = _Canvas(1280 * S, 720 * S)                 # the band furniture, one transparent layer per page type
    under = _Canvas(1280 * S, 720 * S)               # a vector full-page background, painted below that layer
    notes, files, dropped, bgfiles = [], {}, [], []
    pics = [bg[1]] if bg[0] == 'image' and bg[1] else []      # full-bleed rasters, in z-order
    parts = ([master] if master and show_master and master != layout else []) + [layout] \
        + ([rep] if rep is not None and rep != layout else [])
    band = lambda Y, H: (Y + H / 2) / 720 < _BAND_TOP or (Y + H / 2) / 720 > _BAND_BOT
    top_edge, bot_edge = [], []                      # how far the kept furniture reaches into the page
    for part in parts:
        for kind, el, g, rot, fh, fv in tpl.elements(part):
            if g is None or kind not in ('sp', 'pic', 'cxnSp'): continue
            if kind != 'pic' and ''.join(t.text or '' for t in el.iter('{%s}t' % _TNS['a'])).strip():
                continue                             # the template's own words are filler; they are never reused
            X, Y, Wd, Hd = tpl.box(g, small_keep_ratio=(kind == 'pic'))
            fullpage = Wd >= 0.97 * 1280 and Hd >= 0.97 * 720
            if kind == 'pic':
                blip = el.find('.//a:blip', _TNS)
                tgt = tpl.pkg.rels(part).get(blip.get(_T_EMBED)) if blip is not None else None
                if not tgt or tgt[1] not in tpl.pkg.names: continue
                f = tgt[1]
                if fullpage: pics.append(f); continue
                if not (band(Y, Hd) and Wd * Hd <= _BAND_PIC * 1280 * 720):
                    dropped.append(f'版心图片 {Wd:.0f}x{Hd:.0f}'); continue
                ext = f.rsplit('.', 1)[-1].lower()
                if ext == 'emf':
                    r = _emf_ops(tpl.pkg.z.read(f))
                    if r and r[0]: _draw_emf(art, r, X * S, Y * S, Wd * S, Hd * S)
                    else: notes.append(f'{f}: EMF 里没有可绘制的矢量记录，跳过')
                elif ext in ('wmf', 'svg'):
                    notes.append(f'{f}: 不支持 {ext.upper()} 图形，跳过')
                else:
                    dec = _raster(tpl.pkg.z.read(f), ext, max_side=max(Wd, Hd) * S)
                    if dec: art.blit(dec[0], dec[1], dec[2], X * S, Y * S, Wd * S, Hd * S)
                    else: notes.append(f'{f}: {ext} 在这个环境里解不出来，跳过')
                (top_edge if (Y + Hd / 2) / 720 < _BAND_TOP else bot_edge).append((Y, Hd))
                continue
            spPr = el.find('p:spPr', _TNS)
            if spPr is None or spPr.find('a:noFill', _TNS) is not None: continue
            sf = spPr.find('a:solidFill', _TNS)
            fill = _tpl_color(sf, tpl.scheme) if sf is not None else None
            if fill is None:                             # gradient / picture / pattern fill: only ever body art here
                if not fullpage: dropped.append(f'{kind} {Wd:.0f}x{Hd:.0f}')
                continue
            if fullpage:
                # A flat opaque full-page rectangle IS the page colour. Painting it would turn the furniture layer
                # into an opaque sheet that hides the template's own background picture.
                if fill[1] >= 0.99: base = fill[0]
                else: under.fill([[(0, 0), (under.w, 0), (under.w, under.h), (0, under.h)]], fill[0], fill[1])
                continue
            if not (band(Y, Hd) and min(Wd, Hd) <= _BAND_BAR):
                dropped.append(f'{kind} {Wd:.0f}x{Hd:.0f}'); continue
            art.fill([[(X * S, Y * S), ((X + Wd) * S, Y * S), ((X + Wd) * S, (Y + Hd) * S), (X * S, (Y + Hd) * S)]],
                     fill[0], fill[1])
            (top_edge if (Y + Hd / 2) / 720 < _BAND_TOP else bot_edge).append((Y, Hd))
    # Only the backgrounds that can actually be seen: everything under the last opaque raster is hidden by it.
    opq = lambda f: f.rsplit('.', 1)[-1].lower() in ('jpg', 'jpeg', 'bmp')
    cut = max((i for i, f in enumerate(pics) if opq(f)), default=None)
    if cut is not None: pics = pics[cut:]
    seen = set(); pics = [f for f in pics if not (f in seen or seen.add(f))]
    for i, f in enumerate(pics):
        ext = f.rsplit('.', 1)[-1].lower(); data = tpl.pkg.z.read(f)
        if ext in ('emf', 'wmf'):
            # A vector full-page background (the Tencent deck's grid is one): draw it into the layer under the
            # furniture rather than reporting it as lost -- it is the only art some page types have.
            r = _emf_ops(data)
            if r and r[0]: _draw_emf(under, r, 0, 0, under.w, under.h)
            else: notes.append(f'{f}: 整页 {ext.upper()} 里没有可绘制的矢量记录，跳过')
            continue
        if ext not in ('jpg', 'jpeg', 'png'):
            dec = _raster(data, ext, 1600)
            if not dec: notes.append(f'{f}: {ext} 整页底图在这个环境里转不出来，跳过'); continue
            data, ext = _png_encode(*dec), 'png'
        name = f"{role}_bg{'' if i == 0 else i + 1}.{'jpg' if ext in ('jpg', 'jpeg') else 'png'}"
        open(os.path.join(assets, name), 'wb').write(data)
        bgfiles.append((name, _dims_from_bytes(data), _raster(data, ext, 320)))
    if under.used:
        under.blit(art.w, art.h, art.px, 0, 0, art.w, art.h); art = under
    # Composite at 1/8 scale to learn what colour actually sits under the page, for the contrast check.
    comp = _Canvas(320, 180); comp.fill([[(0, 0), (320, 0), (320, 180), (0, 180)]], base)
    guessed = False
    for _n, _d, small in bgfiles:
        if small: comp.blit(small[0], small[1], small[2], 0, 0, 320, 180)
        else: guessed = True
    if art.used: comp.blit(art.w, art.h, art.px, 0, 0, 320, 180)
    if guessed:
        notes.append(f'{bgfiles[0][0]}: 这个环境读不出图片亮度（无 Pillow / sips），页面底色是估的，排版后请核对对比度')
    # The <Slide> colour is hidden behind full-page art, but the contrast check reads it as the surface under every
    # text box without a background of its own. Use the colour covering most of the page, not its mean: a page that
    # is white on top and red below averages to a pink that exists nowhere on it.
    slide_bg = _hexsafe(_comp_mode(comp) if (bgfiles or art.used) else base)
    # The body region clears whatever furniture was kept, and keeps a normal margin where there is none.
    top = max([y + h for y, h in top_edge] + [0.0])
    bot = min([y for y, h in bot_edge] + [720.0])
    top = min(max(top + 24.0, 64.0), 200.0)
    bot = max(min(bot - 24.0, 664.0), top + 300.0)
    area = [64, round(top), 1152, round(bot - top)]
    jsx = [f"<Slide style={{{{ background: '#{slide_bg}', position: 'relative' }}}}>"]
    for name, dims, _s in bgfiles:
        files[name] = {'size': list(dims) if dims else None, 'stretch': bool(dims) and abs(dims[0] / dims[1] - 16 / 9) > 0.03}
        jsx.append(f"    <Image src=\"tpl_assets/{name}\" style={{{{ position: 'absolute', left: 0, top: 0, width: 1280, height: 720 }}}} />")
    if art.used:
        name = f'{role}_art.png'
        open(os.path.join(assets, name), 'wb').write(_png_encode(art.w, art.h, art.px))
        files[name] = {'size': [art.w, art.h], 'stretch': False}
        jsx.append(f"    <Image src=\"tpl_assets/{name}\" style={{{{ position: 'absolute', left: 0, top: 0, width: 1280, height: 720 }}}} />")
    jsx.append(f"    <Box style={{{{ position: 'absolute', left: {area[0]}, top: {area[1]}, width: {area[2]}, height: {area[3]} }}}}>")
    jsx += ['        〔B区内容〕', '    </Box>', '</Slide>']
    return {'jsx': '\n'.join(jsx) + '\n', 'notes': notes, 'files': files, 'area': area,
            'layout': layout, 'layout_name': tpl.name(layout), 'bg': [b[0] for b in bgfiles],
            'art': bool(art.used), 'dropped': dropped, 'surface': _hexsafe(base)}


def _comp_mode(c):
    """The colour covering most of the canvas, quantised to 16 levels per channel and averaged within its bucket."""
    import collections
    buckets = collections.defaultdict(lambda: [0, 0, 0, 0])
    for o in range(0, len(c.px), 4):
        r, g, b = c.px[o], c.px[o + 1], c.px[o + 2]
        k = (r >> 4, g >> 4, b >> 4); e = buckets[k]
        e[0] += r; e[1] += g; e[2] += b; e[3] += 1
    if not buckets: return 'FFFFFF'
    # A gradient scatters across neighbouring buckets, so a small flat area can out-count a large graded one:
    # score each bucket with its neighbours folded in, then average the winning neighbourhood.
    ks = list(buckets)
    near = lambda a, b: all(abs(a[i] - b[i]) <= 2 for i in range(3))
    best = max(ks, key=lambda k: sum(buckets[j][3] for j in ks if near(k, j)))
    tot = [0, 0, 0, 0]
    for j in ks:
        if not near(best, j): continue
        for i in range(4): tot[i] += buckets[j][i]
    return '%02X%02X%02X' % tuple(round(tot[i] / tot[3]) for i in range(3))






def _draw_emf(canvas, r, X, Y, Wd, Hd):
    ops, (x0, y0, x1, y1) = r
    sx, sy = Wd / max(1e-6, x1 - x0), Hd / max(1e-6, y1 - y0)
    tr = lambda p: (X + (p[0] - x0) * sx, Y + (p[1] - y0) * sy)
    for kind, polys, rgb, alpha, eo, w in ops:
        ps = [[tr(p) for p in pp] for pp in polys]
        if kind == 'fill': canvas.fill(ps, rgb, alpha, eo)
        else:
            for pp in ps: canvas.stroke(pp, eo, w * min(sx, sy), rgb, alpha)



def _tpl_design_md(tpl, pages, style):
    """DESIGN.template.md: what the template actually contributes -- its page art, its palette, its type -- plus the
    rules for designing on top of that. It is NOT a position contract: the shells carry no text slots, so there is
    nothing to fill in, only a page to design inside."""
    c = style['colors']; f = style['fonts']; scale = style.get('type_scale') or []
    src = tpl.path.replace('\\', '/').rsplit('/', 1)[-1]
    L = ['# 设计契约（由用户模板生成）', '',
         f'来源：用户模板 {src}；原画幅 {tpl.ratio}' + ('，已横向拉伸为 16:9（徽标、图标等小图保持原比例）' if tpl.ratio != '16:9' else '') + '。',
         '',
         '**模板提供的只有三样：整页底图 / 底色、页眉页脚带里的页面家具（细条、logo）、以及配色与字体。**'
         '版式由你按这三样自己设计——模板里的色块、异形图、卡片、示例文字一律不复刻，也不要在页面上补回来。', '',
         '## 1. 页型外壳', '',
         '| 页型（模板版式） | 外壳 | 本页写的文件 | 整页底图 | 页眉页脚家具 | 可用版心 [x,y,宽,高] |', '|---|---|---|---|---|---|']
    fn = {'cover': '`NN_body_cover.jsx`', 'toc': '`NN_body_toc.jsx`', 'section': '`NN_body_section.jsx`',
          'content': '`NN_body.jsx`', 'end': '`NN_body_end.jsx`'}
    for role in ('cover', 'toc', 'section', 'content', 'end'):
        if role not in pages: continue
        pg = pages[role]; shell = '00_shell.jsx' if role == 'content' else f'00_shell_{role}.jsx'
        L.append(f"| {role}（「{pg['layout_name']}」） | `{shell}` | {fn[role]} | "
                 f"{'、'.join(pg['bg']) or '无（底色 #' + pg['surface'] + '）'} | {'有' if pg.get('art') else '无'} | {pg['area']} |")
    miss = [r for r in ('cover', 'toc', 'section', 'content', 'end') if r not in pages]
    if miss: L += ['', '模板里没有对应版式的页型：' + '、'.join(miss) + '。目录缺 → 用内容页外壳；章节页 / 结束页缺 → 用封面外壳。']
    nobg = [r for r in ('cover', 'toc', 'section', 'content', 'end') if r in pages and not pages[r]['bg'] and not pages[r].get('art')]
    if nobg:
        L += ['', f"⚠️ 这些页型模板里没有底图也没有家具，外壳就是一张底色页：**{'、'.join(nobg)}**。"
                  f"别让它们空着——用主色 {c['primary']} 铺整页或铺一个大色块，再压大号标题 / 章节号，"
                  f"做成和其他页同一套视觉的页面。"]
    L += ['', '## 2. 版心', '',
          '- 每种页型的可用版心见上表最后一列：它已经避开了模板保留下来的页眉页脚家具。**页面上所有内容都落在这个盒子里。**',
          '- `〔B区内容〕` 就在这个盒子里；外壳除此之外没有任何占位，标题、副标题、正文、图表全部由你在这里排。',
          '', '## 3. 色彩（模板里实际涂上去的颜色，按使用次数）', '']
    L.append(f"- 主色 **{c['primary']}**" + (f"，辅色 **{c['secondary']}**" if c['secondary'] else '') + '。')
    chroma = [m['hex'] for m in c['top'] if m['chroma']]; neutral = [m['hex'] for m in c['top'] if not m['chroma']]
    chroma = list(dict.fromkeys('#' + _hexsafe(h[1:]) for h in chroma)); neutral = list(dict.fromkeys('#' + _hexsafe(h[1:]) for h in neutral))
    if chroma: L.append('- 彩色（图表系列、色块、强调按此顺序取）：' + '、'.join(chroma[:8]))
    if neutral: L.append('- 中性色（文字、线、浅面）：' + '、'.join(neutral[:6]) + '；正文深色字用 #1F2329，次级灰字用 #5F6368')
    L.append('- 只用上面这些色值，不加新色相。模板主色配白字过不了对比度检查时，先换 §3 里更深的色值，'
             '再考虑深字配浅色块；最后才把主色压暗到刚好过线，并在交付时说明。一次定下来，不要逐页微调。')
    L += ['', '## 4. 字体与字号', '',
          f"- 标题字体 `{f['title']}`（模板：{f['title_typeface'] or '主题字体'}），正文字体 `{f['body']}`（模板：{f['body_typeface'] or '主题字体'}）；"
          "全篇只用这两种，纯西文数字可用 `Arial`。"]
    if scale:
        L += ['- 下表是**模板自己用的字号档**（px，已按本画幅换算），「模板原文」说明这一档是干什么的：', '',
              '| 字号 | 模板里用了几处 | 模板原文 |', '|---|---|---|']
        L += [f"| **{px}px** | {n} | {'、'.join(eg) or '—'} |" for px, n, eg in scale]
        L += ['', '**从这张表里挑字号，不要自己定。** 挑法：排一页时先定这页的视觉锚点（封面主题 / 章节号 / 核心数字），'
                  '从上表大字档里取一个；正文取「用了十几处以上」的那档；小标题取介于两者之间的一档。'
                  '原文是 `O1`/`01`/`2O2X` 这类编号年份的大字档，是模板的装饰尺度——要用就同样只放编号或数字，别拿它排标题。',
              '- **同一层级全篇同一个字号**，定下来就不再逐页微调。字号不够用时宁可复用已有档，也不要新造中间值。']
    else:
        L += ['- 模板没有显式字号可读（字号全部走主题继承）。按这个层级自己定并全篇统一：'
              '封面 / 章节号 72–120px，页标题 32–48px，小标题 20–24px，正文 15–18px，注释 12–14px。']
    L += ['', '## 5. 怎么排', '',
          '- 版式按内容自由设计（相邻页不重复、非对称优先，见 story-principle.md 版式速查）。',
          '- 视觉语言跟模板走：模板的色块是直角就用直角，没有圆角 / 阴影 / 描边就不加；强调方式照模板（主色色块、色条、编号）。',
          '- **每页一个视觉锚点**：一页上必须有一个明显比其他元素大的东西（大号数字 / 大标题 / 整块主视觉），'
          '其余元素围着它组织。所有元素字号都差不多大 = 流水线页，返工。',
          '- **版心要填满**：行里的块用 `flex: 1` 而不是写死 `width:`；纵向按版心高度反推每块该多厚，填不到八成就并页或拆页，'
          '不要靠空话和拉高固定 height 来凑。`buildall` 会直接报空旷。',
          '- **卡片内部也要填满**：卡片里的内容垂直占到卡高的八成以上；卡越高字越多（高 ≥ 380px 的卡正文不少于 100 字）。'
          '卡尾的标签条 / 引文条用 `marginTop: auto` 钉到底部，别让它浮在卡中间。',
          '- **同一行的几张卡要对齐**：标题块、正文块、尾部条三段在各卡之间高度一致，差出十几像素就看得出来。',
          '- 留白要围着锚点留（锚点周围留出一圈呼吸空间），不要每两个元素之间都留一样大的空 —— 那是稀薄，不是留白。',
          '- 数据用原生 `<Chart>` / `<Table>`，系列颜色按 §3 彩色顺序；表头用主色底白字。',
          '- 不放模板以外的装饰图；模板里的示例照片、示例文字、示例数据一律不用。']
    nts = list(dict.fromkeys([n for r in pages for n in pages[r]['notes']] + tpl.notes))
    drop = sorted({d for r in pages for d in pages[r].get('dropped', [])})
    if nts: L += ['', '## 6. 没能带过来的模板元素（交付时如实说明）', ''] + [f'- {n}' for n in nts]
    if drop:
        L += ['', '## 7. 按设计舍弃的模板装饰（内部用，不必写进交付说明）', '',
              f"- 模板版心里的 {len(drop)} 处色块 / 异形图没有复刻（这是设计选择，不是失败）："
              + '、'.join(drop[:10]) + ('…' if len(drop) > 10 else '')]
    return '\n'.join(L) + '\n'




def template(a):
    """Build the deck's shells from a user's PPT template (see create-from-template.md):
      template <user.pptx> [--dir .]            read the template, write 00_shell*.jsx + tpl_assets/ + DESIGN.template.md
      template --docid <docid> [--dir .]        upload tpl_assets/ to the document and put the urls into the shells
      template --urls tpl_urls.json [--dir .]   same, with urls you uploaded yourself"""
    import os, collections
    d = a.dir; os.makedirs(d, exist_ok=True)
    if a.docid or a.urls: return _tpl_bake(a)
    if not a.pptx: sys.exit('template needs the template file: template <user.pptx>  (or --docid <docid> after the document exists)')
    tpl = _Tpl(a.pptx); assets = os.path.join(d, 'tpl_assets'); os.makedirs(assets, exist_ok=True)
    roles, uses, reps = tpl.roles()
    pages = {}
    for role, layout in roles.items():
        pages[role] = _tpl_page(tpl, role, layout, assets, uses, reps.get(role))
    counter = tpl.painted()
    chroma = [h for h, _n in counter.most_common() if _tpl_chroma(h)]
    primary = chroma[0] if chroma else None
    secondary = next((h for h in chroma[1:] if abs((_rgb_to_hsl(h)[0] - _rgb_to_hsl(primary)[0] + 180) % 360 - 180) >= 30), None) if primary else None
    title_tf = body_tf = None
    for role in ('content', 'cover'):
        if role in pages:
            lay = pages[role]['layout']; m = tpl.master_of.get(lay)
            for kind, el, g, *_ in tpl.elements(lay):
                p = tpl.ph(el) if kind == 'sp' else None
                if p and p[0] in ('title', 'ctrTitle') and title_tf is None: title_tf = tpl.text_style([el, tpl.inherited_ph(m, p)[0]], m, p[0])['typeface']
                if p and p[0] in ('body', 'subTitle') and body_tf is None: body_tf = tpl.text_style([el, tpl.inherited_ph(m, p)[0]], m, p[0])['typeface']
    # The display face of a template is read off its own pages, not off the theme: a deck whose headings are set in
    # one face would otherwise be reported as the theme default and come out in the wrong type throughout. Taking
    # the single largest run is not enough -- the biggest type on a page is often a decorative numeral in a display
    # face used nowhere else (this deck sets 01 / 02 / 03 at 80pt in Impact, and its real headings at 54pt). Among
    # the runs that are large for this template, the face used by the most of them is the one to follow.
    runs = []
    for part in list(tpl.slides) + [p['layout'] for p in pages.values()]:
        root = tpl.pkg.xml(part)
        if root is None: continue
        for r in root.iter('{%s}r' % _TNS['a']):
            rpr = r.find('a:rPr', _TNS)
            if rpr is None: continue
            try: sz = int(rpr.get('sz') or 0)
            except ValueError: continue
            if not sz: continue
            face = rpr.find('a:ea', _TNS)
            if face is None or not (face.get('typeface') or '').strip(): face = rpr.find('a:latin', _TNS)
            tf = (face.get('typeface') or '').strip() if face is not None else ''
            t = r.find('a:t', _TNS)
            runs.append((sz, tf if tf and not tf.startswith('+') else '', (t.text or '').strip() if t is not None else ''))
    big = None
    if runs:
        cut = 0.6 * max(sz for sz, _t, _x in runs)
        tally = collections.Counter(tf for sz, tf, _x in runs if sz >= cut and tf)
        big = tally.most_common(1)[0][0] if tally else None
    tfam = tpl.font(big or title_tf or '+mj-ea')[0]
    bfam = tpl.font(body_tf or '+mn-ea')[0]
    # The template's own type scale, as a plain tally of the sizes it actually sets -- deliberately NOT labelled
    # 'display / heading / body'. Naming them means guessing, and the guess is wrong on real decks: the two biggest
    # steps in this template are its decorative section numerals (O1..O4 at 138pt and 96pt), not a title of any
    # kind. The sample words carry that information without an inference: a step whose samples read 'O1', '02' is
    # ornament, a step with hundreds of runs of prose is the body size. Pick the steps, do not invent sizes.
    steps = collections.defaultdict(lambda: [0, []])
    for sz, _tf, txt in runs:
        px = max(8, round(sz / 100 * tpl.pt))
        key = next((k for k in steps if abs(k - px) <= 2), px)
        steps[key][0] += 1
        if txt and len(steps[key][1]) < 3 and txt not in steps[key][1]: steps[key][1].append(txt[:12])
    # Keep both ends: the largest steps (the template's display scale) and the most-used ones (its body scale).
    # Taking the top N by size alone drops body text entirely -- this deck sets prose at 19px, step 11 of 12.
    keep = set(sorted(steps, reverse=True)[:5]) | set(sorted(steps, key=lambda k: -steps[k][0])[:5])
    scale = [[k, steps[k][0], steps[k][1]] for k in sorted(keep, reverse=True)]
    filler, chars = [], []
    for i, sl in enumerate(tpl.slides, 1):
        txt = [t.text or '' for t in tpl.pkg.xml(sl).iter('{%s}t' % _TNS['a'])]
        if any(_T_FILLER.search(t) for t in txt): filler.append(i)
        chars.append(sum(len(re.sub(r'\s', '', t)) for t in txt if not _T_FILLER.search(t)))
    n = len(tpl.slides); fr = round(len(filler) / n, 2) if n else 1.0
    style = {'file': os.path.abspath(a.pptx), 'canvas': {'ratio': tpl.ratio, 'emu': [tpl.W, tpl.H]},
             'colors': {'primary': '#' + primary if primary else None, 'secondary': '#' + secondary if secondary else None,
                        'top': [{'hex': '#' + h, 'count': c, 'chroma': _tpl_chroma(h)} for h, c in counter.most_common(14)]},
             'fonts': {'title': tfam, 'title_typeface': title_tf, 'body': bfam, 'body_typeface': body_tf},
             'type_scale': scale,
             'text': {'slides': n, 'filler_slides': filler, 'filler_ratio': fr, 'avg_chars': round(sum(chars) / n) if n else 0},
             'kind_hint': 'template' if n == 0 or fr >= 0.3 else 'deck' if fr <= 0.1 else 'mixed',
             'pages': {r: {k: v for k, v in p.items() if k not in ('jsx',)} for r, p in pages.items()}}
    written = []
    for role, pg in pages.items():
        name = '00_shell.jsx' if role == 'content' else f'00_shell_{role}.jsx'
        open(os.path.join(d, name), 'w', encoding='utf-8').write(pg['jsx']); written.append(name)
    open(os.path.join(d, 'DESIGN.template.md'), 'w', encoding='utf-8').write(_tpl_design_md(tpl, pages, style))
    json.dump(style, open(os.path.join(d, 'template_style.json'), 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    print(f"canvas   {tpl.ratio}" + ('  (stretched to 16:9; small art keeps its ratio)' if tpl.ratio != '16:9' else ''))
    print(f"colours  primary {style['colors']['primary']}  secondary {style['colors']['secondary']}   fonts {tfam} / {bfam}")
    for role in ('cover', 'toc', 'section', 'content', 'end'):
        if role not in pages: print(f"{role:8s} -- no layout for it in the template"); continue
        pg = pages[role]
        print(f"{role:8s} layout 「{pg['layout_name']}」 -> {'00_shell.jsx' if role == 'content' else '00_shell_' + role + '.jsx'}  "
              f"版心 {pg['area'][2]}x{pg['area'][3]} @({pg['area'][0]},{pg['area'][1]})"
              + (f"  底图 {', '.join(pg['bg'])}" if pg['bg'] else f"  底色 #{pg['surface']}")
              + ('  页眉页脚家具' if pg.get('art') else ''))
        for nt in pg['notes']: print(f"         note: {nt}")
    print(f"assets   {', '.join(sorted(os.listdir(assets)))}")
    plain = [r for r in ('cover', 'toc', 'section', 'content', 'end') if r in pages and not pages[r]['bg'] and not pages[r].get('art')]
    if plain:
        print(f"\n注意   {', '.join(plain)} 在模板里没有底图也没有页眉页脚家具，外壳就是一张底色页。"
              f"按 DESIGN.template.md §1 的提示用主色 {style['colors']['primary']} 自己做出视觉，别让它空着。")
    print('\n-> DESIGN.template.md, template_style.json, ' + ', '.join(written))
    print('next: DESIGN.template.md is the body of DESIGN.md. After the document exists, upload the art with ONE command:\n'
          '      template --docid <docid>   (then write pages: NN_body*.jsx, and run buildall)\n'
          'the shells carry no text slots: every word on the page is yours to design and place.\n'
          'every text in the template is filler or sample data -- none of it goes into STORY.md as a fact.')


def _tpl_bake(a):
    """Upload tpl_assets/* referenced by the shells and replace the local paths with the returned urls."""
    import os, glob, subprocess, shutil
    d = a.dir; shells = sorted(glob.glob(os.path.join(d, '00_shell*.jsx')))
    used = sorted({m for p in shells for m in re.findall(r'tpl_assets/[\w.\-]+', open(p, encoding='utf-8').read())})
    style = json.load(open(os.path.join(d, 'template_style.json'), encoding='utf-8')) if os.path.exists(os.path.join(d, 'template_style.json')) else {'pages': {}}
    meta = {('tpl_assets/' + k): v for pg in style['pages'].values() for k, v in pg.get('files', {}).items()}
    used = sorted(set(used))
    cache_p = os.path.join(d, 'tpl_urls.json')
    urls = json.load(open(cache_p, encoding='utf-8')) if os.path.exists(cache_p) else {}
    if a.urls: urls.update(json.load(open(a.urls, encoding='utf-8')))
    todo = [u for u in used if u not in urls]
    if todo and a.docid:
        cli = shutil.which(a.cli) or a.cli
        for rel in todo:
            path = os.path.abspath(os.path.join(d, rel))
            r = subprocess.run([cli, 'smartpage', 'images', 'upload', '--json', json.dumps({'docid': a.docid, 'file_path': path})],
                               capture_output=True, text=True)
            m = re.search(r'"url"\s*:\s*"([^"]+)"', r.stdout or '')
            if r.returncode or not m:
                print(f'upload failed for {rel}: {(r.stderr or r.stdout).strip()[:300]}'); continue
            urls[rel] = m.group(1); print(f'uploaded {rel}')
    json.dump(urls, open(cache_p, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    missing = [u for u in used if u not in urls]
    if missing:
        print('\nnot uploaded yet: ' + ', '.join(missing))
        print(f'upload each with  {a.cli} smartpage images upload --json \'{{"docid": "<DOCID>", "file_path": "<abs path>"}}\'\n'
              'put {"tpl_assets/x.png": "<url>"} into a json and rerun: template --urls that.json')
        sys.exit(1)
    for p in shells:
        src = open(p, encoding='utf-8').read(); out = src
        for rel, url in urls.items(): out = out.replace(f'"{rel}"', f'"{url}"')
        if out != src: open(p, 'w', encoding='utf-8').write(out); print(f'ok {os.path.basename(p)}')
    man_p = os.path.join(d, 'images.json'); man = json.load(open(man_p, encoding='utf-8')) if os.path.exists(man_p) else {}
    for rel, url in urls.items():
        m = meta.get(rel) or {}
        if m.get('size'): man[url] = {'width': m['size'][0], 'height': m['size'][1], 'stretch': bool(m.get('stretch'))}
    json.dump(man, open(man_p, 'w', encoding='utf-8'), ensure_ascii=False, indent=1)
    dmd = os.path.join(d, 'DESIGN.template.md')
    if os.path.exists(dmd):
        s = open(dmd, encoding='utf-8').read(); s2 = s
        for rel, url in urls.items(): s2 = s2.replace(f'`{rel}`', f'`{url}`')
        if s2 != s: open(dmd, 'w', encoding='utf-8').write(s2)
    print('\nshells now point at the uploaded art; images.json updated. Write the pages and run buildall.')

ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
sub = ap.add_subparsers(dest='cmd', required=True)
b = sub.add_parser('build'); b.add_argument('--shell', required=True); b.add_argument('--body', required=True); b.add_argument('--vars'); b.add_argument('--manifest', help='images.json; required when the body contains <Image>'); b.add_argument('--out', required=True); b.set_defaults(fn=build)
c = sub.add_parser('check'); c.add_argument('--shell', required=True); c.add_argument('pages', nargs='+'); c.set_defaults(fn=check)
f = sub.add_parser('imgfit'); f.add_argument('--slot', required=True, help='slot WxH, e.g. 576x434'); f.add_argument('--actual', required=True, help='real picture WxH from the generator / upload result'); f.add_argument('--src', required=True); f.add_argument('--bg', help='letterbox colour, e.g. #DCEBFD'); f.set_defaults(fn=imgfit)
g = sub.add_parser('images'); g.add_argument('--manifest', help='json {url: [w, h]} written in Step6'); g.add_argument('pages', nargs='+'); g.set_defaults(fn=images)
r_ = sub.add_parser('recolor'); r_.add_argument('--map', required=True, help='json {"#old": "#new"}'); r_.add_argument('--out-dir', required=True); r_.add_argument('files', nargs='+'); r_.set_defaults(fn=recolor)
i_ = sub.add_parser('init'); i_.add_argument('--set', required=True); i_.add_argument('--out', default='.'); i_.add_argument('--map', help='colors.json for recolouring'); i_.set_defaults(fn=init)
pa = sub.add_parser('palette'); pa.add_argument('--set', required=True); pa.add_argument('--hue', type=float, required=True, help='0-360: 0 red, 30 orange, 45 gold, 90 olive, 140 green, 190 teal, 215 blue, 265 violet, 330 magenta')
pa.add_argument('--sat', type=float, default=1.0, help='saturation multiplier, e.g. 0.7 for a calmer palette'); pa.add_argument('--spread', type=float, default=1.0, help='0-1: how much of the set\'s original hue spread to keep (multi-colour sets)')
pa.add_argument('--invert', action='store_true', help='flip light and dark (light set -> dark scheme)'); pa.add_argument('--out', default='colors.json')
pa.add_argument('--min-gap', dest='min_gap', type=float, default=60.0, help="min degrees between --hue and the set's own hue; inside it, use the set's default palette instead (default 60)")
pa.add_argument('--force', action='store_true', help='derive anyway inside --min-gap (only for a brand colour the user named)'); pa.set_defaults(fn=palette)
tp = sub.add_parser('template'); tp.add_argument('pptx', nargs='?', help="the user's template, .pptx / .potx (convert .ppt first)"); tp.add_argument('--dir', default='.'); tp.add_argument('--docid', help='after the document exists: upload tpl_assets/ to it and put the urls into the shells'); tp.add_argument('--urls', help='json {"tpl_assets/x.png": url} of art you uploaded yourself'); tp.add_argument('--cli', default='wecom-cli'); tp.set_defaults(fn=template)
ba = sub.add_parser('buildall'); ba.add_argument('--dir', default='.'); ba.set_defaults(fn=buildall)
ft = sub.add_parser('fit'); ft.add_argument('pages', nargs='+'); ft.set_defaults(fn=fit)
k = sub.add_parser('contrast'); k.add_argument('pages', nargs='+'); k.set_defaults(fn=contrast)
a = ap.parse_args(); a.fn(a)
