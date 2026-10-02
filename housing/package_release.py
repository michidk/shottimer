"""Package validated housing exports with commit provenance and checksums."""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / 'output' / 'usb-centered'
DIST = ROOT / 'dist'
FILES = ('body.stl', 'cover_print.stl', 'assembly.step', 'fit-check.json',
         'usb-photo-study.json', 'usb_photo_comparison.png', 'body_views.png')


def main():
    sha = os.environ.get('GITHUB_SHA') or subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    repo = os.environ.get('GITHUB_REPOSITORY', 'michidk/shottimer')
    DIST.mkdir(exist_ok=True)
    for name in FILES:
        source = SOURCE / name
        if not source.is_file() or source.stat().st_size == 0:
            raise RuntimeError(f'Missing or empty build output: {source}')
        shutil.copy2(source, DIST / name)
    provenance = {'source_commit': sha, 'repository': repo,
                  'workflow_run': os.environ.get('GITHUB_RUN_ID'),
                  'model': 'housing/usb_centered_stand.py'}
    (DIST / 'build-info.json').write_text(json.dumps(provenance, indent=2) + '\n')
    notes = (f'Latest successful housing build from main.\n\n'
             f'Source commit: [{sha}](https://github.com/{repo}/commit/{sha}).\n\n'
             'Download `housing.zip` for both printable parts, the STEP assembly, '
             'renders and fit reports, or download the STL files individually.\n\n'
             'Print body bottom-down and cover outside-face-down. '
             'This is a development prerelease; physical fit still needs verification.\n')
    (DIST / 'release-notes.md').write_text(notes)
    packaged = (*FILES, 'build-info.json', 'release-notes.md')
    with zipfile.ZipFile(DIST / 'housing.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for name in packaged:
            archive.write(DIST / name, name)
    checksums = ''.join(f'{hashlib.sha256((DIST / name).read_bytes()).hexdigest()}  {name}\n'
                        for name in (*packaged, 'housing.zip'))
    (DIST / 'SHA256SUMS').write_text(checksums)
    print(f'Packaged housing for {sha}')


if __name__ == '__main__':
    main()
