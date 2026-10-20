"""產生 EluGate Conduit 的 App 圖示與系統匣圖示 SVG。

圖示概念：參考低多邊形六角環，色調改為黑灰；中央為 SSH 字樣，
第一個 S 的尾端延伸為向左箭頭、H 的橫線延伸為向右箭頭，代表雙向交互。

執行：python design/generate_icons.py
輸出：design/app-icon.svg、design/tray-*.svg
"""

import math
from pathlib import Path

OUT = Path(__file__).parent
C = 512.0

# 由深到淺的黑灰色階
GREYS = ["#141416", "#1f1f22", "#2c2c30", "#3a3a3f", "#4a4a50", "#5d5d63", "#74747b", "#8e8e95", "#a9a9b0"]

# 每個邊三個三角形的色階索引，模擬參考圖的光影分布（左上較亮、右下與左側較暗）
SHADES = [
    (6, 8, 7),  # 上 → 右上
    (3, 5, 2),  # 右上 → 右下
    (5, 7, 6),  # 右下 → 下
    (2, 4, 1),  # 下 → 左下
    (0, 3, 1),  # 左下 → 左上
    (7, 5, 8),  # 左上 → 上
]

# 系統匣圖示使用較深的色階，讓白字在小尺寸下仍清楚
TRAY_SHADES = [
    (3, 4, 3), (1, 3, 1), (3, 4, 2), (1, 2, 0), (0, 1, 0), (4, 3, 4),
]

IDLE_SHADES = [  # 未連接狀態用的中灰版本
    (5, 6, 5), (4, 5, 4), (5, 6, 5), (4, 5, 4), (4, 4, 4), (6, 5, 6),
]


def hex_points(r):
    return [(C + r * math.cos(math.radians(-90 + 60 * i)), C + r * math.sin(math.radians(-90 + 60 * i))) for i in range(6)]


def pts(*ps):
    return " ".join(f"{x:.1f},{y:.1f}" for x, y in ps)


def facets(outer_r, inner_r, shades, palette=GREYS):
    o = hex_points(outer_r)
    n = hex_points(inner_r)
    out = []
    for i in range(6):
        j = (i + 1) % 6
        m = ((o[i][0] + o[j][0]) / 2, (o[i][1] + o[j][1]) / 2)
        a, b, c = shades[i]
        out.append(f'<polygon points="{pts(o[i], m, n[i])}" fill="{palette[a]}"/>')
        out.append(f'<polygon points="{pts(m, n[j], n[i])}" fill="{palette[b]}"/>')
        out.append(f'<polygon points="{pts(m, o[j], n[j])}" fill="{palette[c]}"/>')
    return "\n    ".join(out)


def ssh_letters(x0, stroke):
    """SSH 字樣：回傳 (中心線路徑, 實心箭頭)；字高 422–602。

    第一個 S 的下方尾端向左延伸成箭頭，H 的右側豎筆從右上角轉向右方射出，
    兩個箭頭呈點對稱，表示雙向交互。
    """
    hh = stroke * 1.7  # 箭頭半高
    ln = stroke * 1.9  # 箭頭長度
    s1 = (
        f"M{x0+112},458 C{x0+100},440 {x0+82},439 {x0+60},439 "
        f"C{x0+30},439 {x0+10},452 {x0+10},475 C{x0+10},498 {x0+30},506 {x0+60},512 "
        f"C{x0+92},518 {x0+112},528 {x0+112},550 C{x0+112},573 {x0+92},585 {x0+60},585 "
        f"L{x0-12},585"
    )
    bx = x0 - 14
    s1_head = f"M{bx},{585-hh} L{bx-ln},585 L{bx},{585+hh} Z"
    x1 = x0 + 150
    s2 = (
        f"M{x1+112},458 C{x1+100},440 {x1+82},439 {x1+60},439 "
        f"C{x1+30},439 {x1+10},452 {x1+10},475 C{x1+10},498 {x1+30},506 {x1+60},512 "
        f"C{x1+92},518 {x1+112},528 {x1+112},550 C{x1+112},573 {x1+92},585 {x1+60},585 "
        f"C{x1+36},585 {x1+18},577 {x1+8},560"
    )
    xh = x0 + 300
    h = (
        f"M{xh+10},439 L{xh+10},585 M{xh+10},512 L{xh+100},512 "
        f"M{xh+100},585 L{xh+100},439 L{xh+152},439"
    )
    hx = xh + 150
    h_head = f"M{hx},{439-hh} L{hx+ln},439 L{hx},{439+hh} Z"
    return [s1, s2, h], [s1_head, h_head]


def letters_group(color, stroke, scale):
    # 左右箭頭以 x0+218 為中心對稱，使字樣水平置中
    x0 = 512 - 218
    lines, heads = ssh_letters(x0, stroke)
    paths = "\n      ".join(f'<path d="{d}"/>' for d in lines)
    fills = "\n      ".join(f'<path d="{d}" fill="{color}" stroke="none"/>' for d in heads)
    return (
        f'<g transform="translate(512 512) scale({scale}) translate(-512 -512)" fill="none" '
        f'stroke="{color}" stroke-width="{stroke}" stroke-linecap="square" stroke-linejoin="miter" '
        f'stroke-miterlimit="10">\n      {paths}\n      {fills}\n    </g>'
    )


def app_icon():
    inner = hex_points(298)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <title>EluGate Conduit</title>
  <g>
    {facets(488, 298, SHADES)}
  </g>
  <polygon points="{pts(*inner)}" fill="#f4f4f6"/>
  {letters_group("#1c1c1f", 40, 0.72)}
</svg>
"""


STATUS_COLORS = {
    "connected": "#22c55e",
    "reconnecting": "#f59e0b",
    "error": "#ef4444",
}


def tray_icon(status):
    # 系統匣尺寸很小：使用實心六角形 + 白色粗字，提高辨識度
    light = status == "idle"
    shades = IDLE_SHADES if light else TRAY_SHADES
    body = facets(500, 0.01, shades)
    badge = ""
    if status in STATUS_COLORS:
        badge = (
            f'<circle cx="812" cy="812" r="200" fill="#ffffff"/>'
            f'<circle cx="812" cy="812" r="160" fill="{STATUS_COLORS[status]}"/>'
        )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <g>
    {body}
  </g>
  {letters_group("#ffffff", 60, 0.86)}
  {badge}
</svg>
"""


def main():
    (OUT / "app-icon.svg").write_text(app_icon(), encoding="utf-8")
    for status in ["idle", "connected", "reconnecting", "error"]:
        (OUT / f"tray-{status}.svg").write_text(tray_icon(status), encoding="utf-8")
    print("icons written to", OUT)


if __name__ == "__main__":
    main()
