#!/usr/bin/env python3
"""Generate minimal EPUB test samples for each supported language."""
import os, zipfile, textwrap

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(SCRIPT_DIR, "..", "kothok", "crates", "kobo", "samples")

LANGS = {
    "en": {
        "title": "The Happy Prince",
        "author": "Oscar Wilde",
        "lang": "en",
        "paragraphs": [
            "One night there flew over the city a little Swallow. His friends had gone away to Egypt six weeks before, but he had stayed behind, for he was in love with the most beautiful Reed.",
            "He had met her early in the spring as he was flying down the river after a big yellow moth, and had been so attracted by her slender waist that he had stopped to talk to her.",
            "Shall I love you? said the Swallow, who liked to come to the point at once, and the Reed made him a low bow. So he flew round and round her, touching the water with his wings, and making silver ripples.",
            "This was his courtship, and it lasted all through the summer. It is a ridiculous attachment, twittered the other Swallows, she has no money, and far too many relations.",
        ],
    },
    "bn": {
        "title": "কবিতা সংকলন",
        "author": "রবীন্দ্রনাথ ঠাকুর",
        "lang": "bn",
        "paragraphs": [
            "আমার মা, তোমার চরণতলে দিনমণি আজ তোমার রঙে রঙিত করি।",
            "বাংলার মুখ আমি দেখিয়াছি, তাই আমি এ বিশ্ব রূপ খুঁজিয়া ফেরি না।",
            "তোমার সৃষ্টির পথ ধরিয়া একা আমি চলিয়াছি আমার মন।",
            "যেথা আকাশ ভেঙে পড়ে ভূমির কোলে, সেথা আমার বাংলা ভাষা জাগে।",
        ],
    },
    "hi": {
        "title": "नीति कथा",
        "author": "परंपरा",
        "lang": "hi",
        "paragraphs": [
            "एक समय की बात है, एक गाँव में एक बूढ़ा आदमी रहता था। वह बहुत बुद्धिमान था।",
            "उसके तीन बेटे थे। वे आपस में हमेशा झगड़ा करते थे। बूढ़ा आदमी उन्हें समझाता, पर वे नहीं मानते।",
            "एक दिन बूढ़े आदमी ने एक लकड़ी का गट्ठर लाया और बेटों से कहा, इसे तोड़ो।",
            "सबने मिलकर भी उसे नहीं तोड़ पाए। फिर बूढ़े ने एक-एक लकड़ी निकाली और आसानी से तोड़ दी।",
        ],
    },
    "ar": {
        "title": "كليلة ودمنة",
        "author": "ابن المقفع",
        "lang": "ar",
        "paragraphs": [
            "قال الفيلسوف: ينبغي للعاقل أن يكون حافظاً لسانه، مقدماً لكلامه قبل أن ينطق به.",
            "ومن تأدب بآداب العلماء، عرف قدر نفسه، ولم يتكبر على أحد من الناس.",
            "والكلمة الطيبة صدقة، فمن أكثر من القول أخطأ، ومن أحسن في عمله أفلح.",
            "اعلم أن الصبر مفتاح الفرج، وأن الشكر زيادة في النعم، وأن الكبر هلاك للنفس.",
        ],
    },
    "ja": {
        "title": "ごん狐",
        "author": "新美南吉",
        "lang": "ja",
        "paragraphs": [
            "これは、わたしが小さいときに、村の茂平というおじいさんからきいたお話です。",
            "ごんという、ひとりの狐がいました。ごんは、村の小さな原っぱに住んでいました。",
            "ある日、ごんは村の兵十という男が、川で魚をとっているのを見ました。",
            "兵十は、お母さんが病気なので、おかゆにいれてやるために魚をとったのでした。",
        ],
    },
    "th": {
        "title": "นิทานธรรมะ",
        "author": "ประเพณี",
        "lang": "th",
        "paragraphs": [
            "กาลครั้งหนึ่งนะน้อง มีกระต่ายตัวหนึ่งอาศัยอยู่ในป่า มันมีหูยาวและขนสวย",
            "วันหนึ่งกระต่ายไปพบเต่าที่กำลังเดินช้ามาก กระต่ายจึงหัวเราะเยาะเต่า",
            "เต่าไม่โกรธ แต่ท้าแข่งวิ่ง กระต่ายหัวเราะและตอบตกลงทันที",
            "กระต่ายวิ่งได้ไกลและคิดว่าชนะแน่ จึงหยุดนอนหลับ ส่วนเต่าเดินต่อไปเรื่อย",
        ],
    },
}

def make_epub(lang, data):
    fname = f"{lang}-sample.epub"
    path = os.path.join(OUT, fname)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("mimetype", "application/epub+zip", zipfile.ZIP_STORED)
        z.writestr("META-INF/container.xml", '''<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>''')
        z.writestr("OEBPS/content.opf", f'''<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>{data["title"]}</dc:title>
    <dc:creator>{data["author"]}</dc:creator>
    <dc:language>{data["lang"]}</dc:language>
    <dc:identifier id="bookid">kothok-{lang}-sample</dc:identifier>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
  </spine>
</package>''')
        z.writestr("OEBPS/toc.ncx", f'''<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head><meta name="dtb:uid" content="kothok-{lang}-sample"/></head>
  <docTitle><text>{data["title"]}</text></docTitle>
  <navMap><navPoint playOrder="1" id="ch1">
    <navLabel><text>Chapter 1</text></navLabel>
    <content src="chapter1.xhtml"/>
  </navPoint></navMap>
</ncx>''')
        paras = "\n".join(f"    <p>{p}</p>" for p in data["paragraphs"])
        z.writestr("OEBPS/chapter1.xhtml", f'''<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="{data["lang"]}" lang="{data["lang"]}">
<head><meta charset="UTF-8"/><title>{data["title"]}</title></head>
<body>
  <h1>{data["title"]}</h1>
  <h2>{data["author"]}</h2>
{paras}
</body>
</html>''')
    size = os.path.getsize(path)
    print(f"  {fname:20s} {size:>6,} bytes")

print("Generating EPUB samples...")
for lang, data in LANGS.items():
    make_epub(lang, data)
print("Done.")
