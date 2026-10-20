"""產生 Windows 安裝程式（MSI 與 NSIS）使用的圖片。

- wix-dialog.bmp 493×312：MSI 歡迎與完成頁的背景；左側 164px 為圖片區，右側由安裝程式顯示文字
- wix-banner.bmp 493×58：MSI 其他頁面頂端橫幅；左側由安裝程式顯示標題，圖示放在右側
- nsis-sidebar.bmp 164×314：NSIS 歡迎與完成頁左側圖片
- nsis-header.bmp 150×57：NSIS 其他頁面右上角圖片

執行：python design/generate_installer_images.py（需要 Pillow；先執行 generate_icons.py 與 npm run icons）
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
ICON = ROOT / "src-tauri" / "icons" / "128x128@2x.png"
OUT = ROOT / "src-tauri" / "installer"

SIDE_W = 164
DARK = (24, 24, 27)
DARK2 = (44, 44, 49)
LINE = (70, 70, 77)
TEXT = (236, 236, 240)
MUTED = (150, 150, 158)


def font(size, bold=False):
    names = ["segoeuib.ttf", "arialbd.ttf"] if bold else ["segoeui.ttf", "arial.ttf"]
    for n in names:
        try:
            return ImageFont.truetype(str(Path("C:/Windows/Fonts") / n), size)
        except OSError:
            continue
    return ImageFont.load_default()


def hexagon(cx, cy, r):
    return [(cx + r * math.cos(math.radians(-90 + 60 * i)), cy + r * math.sin(math.radians(-90 + 60 * i))) for i in range(6)]


def logo(size):
    return Image.open(ICON).convert("RGBA").resize((size, size), Image.LANCZOS)


def sidebar(height):
    """深灰漸層側欄，放圖示與名稱；MSI 與 NSIS 共用"""
    side = Image.new("RGB", (SIDE_W, height))
    d = ImageDraw.Draw(side)
    for y in range(height):
        t = y / (height - 1)
        c = tuple(round(DARK2[i] * (1 - t) + DARK[i] * t) for i in range(3))
        d.line([(0, y), (SIDE_W, y)], fill=c)
    # 低調的六角形線條作為背景紋理
    for cx, cy, r in [(158, 22, 62), (12, height + 6, 74)]:
        d.polygon(hexagon(cx, cy, r), outline=LINE)
    mark = logo(104)
    side.paste(mark, ((SIDE_W - 104) // 2, 64), mark)
    f1, f2 = font(17, bold=True), font(12)
    for text, f, y, color in [("EluGate", f1, 186, TEXT), ("Conduit", f1, 208, TEXT), ("SSH Tunnel Manager", f2, 238, MUTED)]:
        w = d.textlength(text, font=f)
        d.text(((SIDE_W - w) / 2, y), text, font=f, fill=color)
    return side


def dialog():
    img = Image.new("RGB", (493, 312), (255, 255, 255))
    img.paste(sidebar(312), (0, 0))
    return img


def banner():
    img = Image.new("RGB", (493, 58), (255, 255, 255))
    d = ImageDraw.Draw(img)
    # 右側深色區塊放置圖示，取代預設的紅色光碟圖案
    x0 = 493 - 70
    d.rectangle([x0, 0, 493, 58], fill=DARK2)
    d.polygon([(x0 - 14, 0), (x0, 0), (x0, 58)], fill=DARK2)
    mark = logo(46)
    img.paste(mark, (x0 + (70 - 46) // 2, 6), mark)
    return img


def nsis_header():
    # NSIS 把此圖放在頁首右側，標題文字在其左方
    img = Image.new("RGB", (150, 57), (255, 255, 255))
    d = ImageDraw.Draw(img)
    x0 = 150 - 66
    d.rectangle([x0, 0, 150, 57], fill=DARK2)
    d.polygon([(x0 - 14, 0), (x0, 0), (x0, 57)], fill=DARK2)
    mark = logo(45)
    img.paste(mark, (x0 + (66 - 45) // 2, 6), mark)
    return img


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    dialog().save(OUT / "wix-dialog.bmp")
    banner().save(OUT / "wix-banner.bmp")
    sidebar(314).save(OUT / "nsis-sidebar.bmp")
    nsis_header().save(OUT / "nsis-header.bmp")
    print("installer images written to", OUT)


if __name__ == "__main__":
    main()
