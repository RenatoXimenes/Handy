"""Generate original Vozel tray assets from a simple waveform mark.

Run after installing Pillow: python3 scripts/generate-vozel-tray-icons.py
The app icon is generated separately with `bun tauri icon src/assets/vozel-icon.svg`.
"""

from pathlib import Path

from PIL import Image, ImageDraw


DEST = Path(__file__).resolve().parents[1] / "src-tauri" / "resources"
SCALE = 4
POINTS = [(6, 32), (13, 32), (19, 17), (28, 48), (37, 17), (43, 32), (53, 32)]


def draw_mark(color: str, status: str | None, filled: bool) -> Image.Image:
    image = Image.new("RGBA", (64 * SCALE, 64 * SCALE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    if filled:
        draw.rounded_rectangle((2 * SCALE, 2 * SCALE, 62 * SCALE, 62 * SCALE), radius=15 * SCALE, fill="#142831")
    draw.line([(x * SCALE, y * SCALE) for x, y in POINTS], fill=color, width=5 * SCALE, joint="curve")
    for x, y in (POINTS[0], POINTS[-1]):
        draw.ellipse(((x - 2.5) * SCALE, (y - 2.5) * SCALE, (x + 2.5) * SCALE, (y + 2.5) * SCALE), fill=color)
    if status:
        draw.ellipse((47 * SCALE, 43 * SCALE, 62 * SCALE, 58 * SCALE), fill="#142831" if filled else "#526168")
        draw.ellipse((50 * SCALE, 46 * SCALE, 59 * SCALE, 55 * SCALE), fill=status)
    return image.resize((64, 64), Image.Resampling.LANCZOS)


def main() -> None:
    states = {"idle": None, "recording": "#FF776B", "transcribing": "#FFC86B", "idle_warning": "#FFC86B"}
    for state, badge in states.items():
        draw_mark("#F7FAFA", badge, False).save(DEST / f"vozel_tray_{state}.png")
        draw_mark("#1C4148", badge, False).save(DEST / f"vozel_tray_{state}_dark.png")
    for state, badge in states.items():
        draw_mark("#55D6C2", badge, True).save(DEST / f"vozel_{state}.png")


if __name__ == "__main__":
    main()
