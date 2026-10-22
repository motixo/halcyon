#!/usr/bin/env python3
"""Generate coordinated gradient wallpapers and previews without network assets."""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image

THEMES = {
    'lavender': ('171723', 'b8a2ed', '6a669f'),
    'amber': ('26211e', 'e9b887', 'ad745b'),
    'teal': ('07171b', '6ee7ce', '288c9e'),
    'ocean': ('101b2a', '88bdff', '4268a5'),
    'rose': ('241821', 'f2acc0', 'a15b79'),
    'graphite': ('181b20', 'c5cdd8', '687789'),
}


def rgb(value):
    return np.array([int(value[i:i + 2], 16) for i in (0, 2, 4)], dtype=np.float32)


def render(name, width=3840, height=2160):
    base, light, second = THEMES[name]
    y, x = np.mgrid[0:height, 0:width].astype(np.float32)
    x /= width
    y /= height
    pixels = np.zeros((height, width, 3), dtype=np.float32) + rgb(base)
    # Broad pools of light keep the center quiet for desktop content.
    glow = np.exp(-(((x - .27) / .44) ** 2 + ((y - 1.06) / .61) ** 2) * 1.7)
    pixels += glow[..., None] * (rgb(light) - rgb(base)) * .32
    glow2 = np.exp(-(((x - .93) / .42) ** 2 + ((y - .63) / .73) ** 2) * 2.)
    pixels += glow2[..., None] * (rgb(second) - rgb(base)) * .13
    # Two subtle curved folds along the foot of the desktop.
    ridge = .96 - .15 * np.cos((x - .08) * 3.1)
    fold = 1 / (1 + np.exp(np.clip(-(y - ridge) * 110, -80, 80)))
    pixels -= fold[..., None] * 3.5
    edge = np.exp(-((y - ridge) / .006) ** 2)
    pixels += edge[..., None] * .6
    ridge2 = 1.12 - .18 * np.sin(x * 3.3)
    fold2 = 1 / (1 + np.exp(np.clip(-(y - ridge2) * 95, -80, 80)))
    pixels += fold2[..., None] * 2.
    # Reproducible fine grain prevents visible gradient bands on dark displays.
    pixels += np.random.default_rng(42).normal(0, .48, (height, width, 1)).astype(np.float32)
    return Image.fromarray(np.clip(np.round(pixels), 0, 255).astype(np.uint8))


def generate(names, output, previews):
    output.mkdir(parents=True, exist_ok=True)
    previews.mkdir(parents=True, exist_ok=True)
    for name in names:
        result = render(name)
        result.save(output / f'{name}-4k.png', optimize=True)
        result.resize((1920, 1080), Image.Resampling.LANCZOS).save(
            output / f'{name}-1080p.png', optimize=True)
        for suffix, size in [('preview', (960, 540)), ('thumb', (320, 180))]:
            result.resize(size, Image.Resampling.LANCZOS).save(
                previews / f'{name}-{suffix}.png', optimize=True)
        print(output / f'{name}-4k.png')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--themes', choices=THEMES, nargs='+', default=list(THEMES))
    parser.add_argument('--output', type=Path, default=Path.home() / 'Pictures' / 'Desktop Cards')
    parser.add_argument('--previews', type=Path,
                        default=Path(__file__).resolve().parent.parent / 'assets' / 'previews')
    args = parser.parse_args()
    generate(args.themes, args.output, args.previews)


if __name__ == '__main__':
    main()
