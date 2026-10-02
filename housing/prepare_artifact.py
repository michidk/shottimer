"""Collect only the printable CAD files, named by PCB and housing variant."""
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent
OUTPUT = ROOT / 'output'
ARTIFACT = OUTPUT / 'artifact'
VARIANTS = {
    'rp2040-lcd-1.28-desktop-stand': (
        OUTPUT / 'usb-centered', 'body.stl'),
    'rp2040-lcd-1.28-magnetic-usb-left-magnet-right': (
        OUTPUT / 'magnetic' / 'usb-left_magnet-right', 'body_print.stl'),
}


def prepare():
    files = []
    for variant, (source, body) in VARIANTS.items():
        for original, suffix in ((body, 'body.stl'),
                                 ('cover_print.stl', 'cover.stl'),
                                 ('assembly.step', 'assembly.step')):
            src = source / original
            if not src.is_file() or not src.stat().st_size:
                raise RuntimeError(f'Missing or empty CAD export: {src}')
            files.append((src, ARTIFACT / variant / f'{variant}-{suffix}'))
    # This directory contains only copies of generated exports.
    if ARTIFACT.exists():
        shutil.rmtree(ARTIFACT)
    for src, dest in files:
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
        print(dest.relative_to(ARTIFACT))


if __name__ == '__main__':
    prepare()
