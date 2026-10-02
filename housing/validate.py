"""Validate exported geometry and render it with Flowful's preview tool."""
from pathlib import Path
import json
import sys
import numpy as np
np.infty = np.inf  # compatibility for pyrender 0.1.45 + NumPy 2
import trimesh
from model import build

root=Path(__file__).parent
report={}
for path in (root/'output').glob('*.stl'):
    m=trimesh.load(path,force='mesh')
    assert m.is_watertight and m.is_volume, str(path)
    report[path.name]={'watertight':bool(m.is_watertight),'volume_mm3':round(float(m.volume),2)}
for tilt in (0,10,20,30,40):
    report[f'tilt_{tilt}']=build(tilt)['report']['overall_mm']
(root/'output'/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))

skill=Path.home()/'.codex/skills/parametric-3d-printing'
sys.path.insert(0,str(skill))
import preview
m=preview.load_mesh(str(root/'output'/'concept_assembly.stl'))
assert m.is_watertight
preview.render_multi_view(m,str(root/'output'/'technical_preview.png'))
