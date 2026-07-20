#!/usr/bin/env python3
"""Build a single EPUB carrying one passage per supported script.

Rendering is the thing worth testing here, and it is per-run: `detect_script`
classifies each paragraph independently, so one mixed book exercises every face
and every fallback path in a single open. A missing face shows up immediately as
a row of blank boxes next to its label.

With --deploy, also copies the EPUB to <kobo>/.adds/kothok after building, auto-
detecting the USB mount the same way kothok/scripts/deploy.ps1 does (a .adds or
.kobo marker at the drive root). An MD5 mismatch after copy aborts.

The script names mirror `Script` in kobo-core; the font column names the face
that must be present in `.adds/fonts` for that row to render.
"""
import argparse
import hashlib
import os
import shutil
import string
import sys
import zipfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(SCRIPT_DIR, "..", "kothok", "samples", "script-test.epub"))
# Device destination: the app folder, NOT samples/ (which KoThok scans for
# books and the test epub would pollute the library).
DEVICE_DEST = ".adds/kothok"

# (script label, required font file, sample text)
SAMPLES = [
    ("Latin", "embedded", "The quick brown fox jumps over the lazy dog."),
    ("Latin / Vietnamese", "embedded", "Xin chao cac ban, toi dang doc mot cuon sach hay."),
    ("Greek", "NotoSans.ttf", "Καλημέρα κόσμε, διαβάζω ένα βιβλίο σήμερα."),
    ("Cyrillic", "NotoSans.ttf", "Привет мир, я читаю книгу сегодня вечером."),
    ("Hebrew", "NotoSansHebrew.ttf", "שלום עולם, אני קורא ספר טוב היום."),
    ("Arabic", "NotoSansArabic.ttf", "مرحبا بالعالم، أنا أقرأ كتابا جميلا اليوم."),
    ("Bengali", "NotoSansBengali.ttf", "নমস্কার বিশ্ব, আমি আজ একটি সুন্দর বই পড়ছি।"),
    ("Devanagari", "NotoSansDevanagari.ttf", "नमस्ते दुनिया, मैं आज एक अच्छी किताब पढ़ रहा हूँ।"),
    ("Gujarati", "NotoSansGujarati.ttf", "નમસ્તે વિશ્વ, હું આજે એક સરસ પુસ્તક વાંચું છું."),
    ("Gurmukhi", "NotoSansGurmukhi.ttf", "ਸਤ ਸ੍ਰੀ ਅਕਾਲ, ਮੈਂ ਅੱਜ ਇੱਕ ਕਿਤਾਬ ਪੜ੍ਹ ਰਿਹਾ ਹਾਂ।"),
    ("Tamil", "NotoSansTamil.ttf", "வணக்கம் உலகம், நான் இன்று ஒரு நல்ல புத்தகம் படிக்கிறேன்."),
    ("Telugu", "NotoSansTelugu.ttf", "నమస్కారం ప్రపంచం, నేను ఈరోజు ఒక పుస్తకం చదువుతున్నాను."),
    ("Kannada", "NotoSansKannada.ttf", "ನಮಸ್ಕಾರ ಜಗತ್ತು, ನಾನು ಇಂದು ಒಂದು ಪುಸ್ತಕ ಓದುತ್ತಿದ್ದೇನೆ."),
    ("Malayalam", "NotoSansMalayalam.ttf", "നമസ്കാരം ലോകം, ഞാൻ ഇന്ന് ഒരു പുസ്തകം വായിക്കുന്നു."),
    ("Sinhala", "NotoSansSinhala.ttf", "ආයුබෝවන් ලෝකය, මම අද පොතක් කියවනවා."),
    ("Thai", "NotoSansThai.ttf", "สวัสดีชาวโลก วันนี้ฉันกำลังอ่านหนังสือที่ดีมาก"),
    ("Lao", "NotoSansLao.ttf", "ສະບາຍດີຊາວໂລກ ມື້ນີ້ຂ້ອຍກຳລັງອ່ານປຶ້ມ."),
    ("Khmer", "NotoSansKhmer.ttf", "ជំរាបសួរពិភពលោក ថ្ងៃនេះខ្ញុំកំពុងអានសៀវភៅ។"),
    ("Myanmar", "NotoSansMyanmar.ttf", "မင်္ဂလာပါကမ္ဘာလောက၊ ဒီနေ့ ကျွန်တော် စာအုပ်ဖတ်နေပါတယ်။"),
    ("Georgian", "NotoSansGeorgian.ttf", "გამარჯობა მსოფლიო, დღეს წიგნს ვკითხულობ."),
    ("Armenian", "NotoSansArmenian.ttf", "Բարեւ աշխարհ, այսօր ես գիրք եմ կարդում."),
    ("Ethiopic", "NotoSansEthiopic.ttf", "ሰላም ዓለም፣ ዛሬ አንድ መጽሐፍ እያነበብኩ ነው።"),
    ("Japanese", "NotoSansJP.ttf", "こんにちは世界、今日はいい本を読んでいます。"),
    ("Korean", "NotoSansKR.ttf", "안녕하세요 세계, 오늘 좋은 책을 읽고 있습니다."),
    ("Chinese", "NotoSansSC.ttf", "你好世界，我今天在读一本好书。"),
]

def find_kobo():
    """Return the mounted Kobo onboard root, or None.

    Mirrors Find-Kobo in kothok/scripts/deploy.ps1: any filesystem root that
    carries a .adds or .kobo marker is treated as the device.
    """
    if sys.platform == "win32":
        for letter in string.ascii_uppercase:
            root = f"{letter}:\\"
            if _looks_like_kobo(root):
                return root
        return None

    bases = ["/Volumes"] if sys.platform == "darwin" else ["/media", "/run/media"]
    for base in bases:
        if not os.path.isdir(base):
            continue
        for user in os.listdir(base):
            user_dir = os.path.join(base, user)
            if not os.path.isdir(user_dir):
                continue
            for name in os.listdir(user_dir):
                mount = os.path.join(user_dir, name)
                if _looks_like_kobo(mount):
                    return mount
    return None


def _looks_like_kobo(root):
    return os.path.isdir(root) and (
        os.path.isdir(os.path.join(root, ".adds"))
        or os.path.isdir(os.path.join(root, ".kobo"))
    )


def md5_of(path):
    h = hashlib.md5()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def deploy(kobo_root):
    dest_dir = os.path.join(kobo_root, DEVICE_DEST)
    os.makedirs(dest_dir, exist_ok=True)
    target = os.path.join(dest_dir, os.path.basename(OUT))
    shutil.copy2(OUT, target)
    src = md5_of(OUT)
    dst = md5_of(target)
    if src != dst:
        print(f"ERROR: MD5 mismatch after copy (src={src} dst={dst})", file=sys.stderr)
        sys.exit(1)
    print(f"deployed -> {target} ({os.path.getsize(target)} bytes, md5 {src})")


CONTAINER = """<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>
"""


def build():
    body = []
    for label, font, text in SAMPLES:
        body.append(f"    <h2>{label}</h2>")
        body.append(f'    <p class="face">{font}</p>')
        body.append(f"    <p>{text}</p>")
    body = "\n".join(body)

    chapter = f"""<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head>
    <title>Script Test</title>
    <style>.face {{ color: #777; font-size: 0.8em; }}</style>
  </head>
  <body>
    <h1>Script rendering test</h1>
    <p>Every passage below is set in a different script. A row of blank boxes
       means the face named under its heading is missing from
       .adds/fonts, or failed to load.</p>
{body}
  </body>
</html>
"""

    opf = """<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Script Rendering Test</dc:title>
    <dc:creator>KoThok</dc:creator>
    <!-- Deliberately undeclared beyond English: every passage must be resolved
         by detect_script, not by the book's language tag. -->
    <dc:language>en</dc:language>
    <dc:identifier id="bookid">kothok-script-test</dc:identifier>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx"><itemref idref="ch1"/></spine>
</package>
"""

    ncx = """<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head><meta name="dtb:uid" content="kothok-script-test"/></head>
  <docTitle><text>Script Rendering Test</text></docTitle>
  <navMap>
    <navPoint id="n1" playOrder="1">
      <navLabel><text>Scripts</text></navLabel>
      <content src="chapter1.xhtml"/>
    </navPoint>
  </navMap>
</ncx>
"""

    if os.path.exists(OUT):
        os.remove(OUT)
    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
        # mimetype must be first and stored uncompressed.
        z.writestr(zipfile.ZipInfo("mimetype"), "application/epub+zip",
                   compress_type=zipfile.ZIP_STORED)
        z.writestr("META-INF/container.xml", CONTAINER)
        z.writestr("OEBPS/content.opf", opf)
        z.writestr("OEBPS/toc.ncx", ncx)
        z.writestr("OEBPS/chapter1.xhtml", chapter)

    print(f"wrote {OUT} ({os.path.getsize(OUT)} bytes, {len(SAMPLES)} passages)")


def main():
    parser = argparse.ArgumentParser(
        description="Build a multi-script EPUB for font/render testing on Kobo.",
    )
    parser.add_argument(
        "--deploy", action="store_true",
        help=f"copy the built EPUB to <kobo>/{DEVICE_DEST} after building",
    )
    parser.add_argument(
        "--kobo", metavar="PATH",
        help="explicit Kobo mount root (default: auto-detect via .adds/.kobo marker)",
    )
    args = parser.parse_args()

    build()

    if args.deploy:
        kobo = args.kobo or find_kobo()
        if not kobo:
            print("ERROR: Kobo not found. Plug it in or pass --kobo PATH.",
                  file=sys.stderr)
            sys.exit(1)
        print(f"kobo: {kobo}")
        deploy(kobo)


if __name__ == "__main__":
    main()
