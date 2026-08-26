#!/usr/bin/env python3
"""Generate Nether's pixel-art nether-portal icon assets.

Draws a 16x16 chunky pixel-art Minecraft-style nether portal (obsidian frame,
swirling purple portal fill, a few bright sparkles), then upscales it with
nearest-neighbour resampling so pixels stay razor sharp.
"""

from PIL import Image

SIZE = 16

# palette
T = (0, 0, 0, 0)            # transparent
D = (11, 6, 20, 255)        # obsidian deepest
O = (24, 15, 40, 255)       # obsidian base
o = (42, 27, 71, 255)       # obsidian highlight
H = (61, 40, 101, 255)      # obsidian rim light
p = (46, 18, 82, 255)       # portal darkest
P = (94, 38, 168, 255)      # portal deep
q = (143, 69, 232, 255)     # portal mid
r = (171, 108, 247, 255)    # portal light
s = (217, 184, 255, 255)    # sparkle
w = (239, 224, 255, 255)    # hot sparkle core

PAL = {
    ".": T, "D": D, "O": O, "o": o, "H": H,
    "p": p, "P": P, "q": q, "r": r, "s": s, "w": w,
}

# Hand-authored 16x16 grid. Frame = obsidian with irregular sheen; interior =
# diagonal swirl bands from dark (bottom-left) to bright (top-right).
GRID = [
    "DDOOOoOOOOOoOOOD",
    "DOOOOOoOOOOOOOoD",
    "DOrqqrrrqrsqqrqD",
    "DOrqrrqsrqqqrrqD",
    "DOqqrrqqqrrqsqPD",
    "DOqqrsqrrqqqqPPD",
    "DOsqPqqqrrqPPPpD",
    "DOPPqqsqPqPPPppD",
    "DOPPqqPqPPPsPppD",
    "DOPPPqPPPPPPPppD",
    "DsPPPqPsPPPPpppD",
    "DPsPPPPPPPsPpppD",
    "DOPPpsPPPPpppOpD",
    "DOOPPpPpppppOOoD",
    "DOOOOpppppOOOOoD",
    "DDOOOOOOOOOOOoDD",
]

assert len(GRID) == SIZE and all(len(row) == SIZE for row in GRID), "grid must be 16x16"


def render() -> Image.Image:
    img = Image.new("RGBA", (SIZE, SIZE))
    px = img.load()
    for y, row in enumerate(GRID):
        for x, ch in enumerate(row):
            px[x, y] = PAL[ch]
    return img


def upscale(img: Image.Image, factor: int) -> Image.Image:
    return img.resize((img.width * factor, img.height * factor), Image.NEAREST)


def main() -> None:
    art = render()

    master = upscale(art, 64)  # 1024x1024 source for tauri icon
    master.save("icons/nether-1024.png")

    upscale(art, 32).save("public/portal.png")          # UI logo / favicon
    upscale(art, 12).save("icons/nether-192.png")

    print("wrote icons/nether-1024.png, icons/nether-192.png, public/portal.png")


if __name__ == "__main__":
    main()
