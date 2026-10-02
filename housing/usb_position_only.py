"""Restore proven USB opening size; shift it 3 mm away from front."""
from pathlib import Path
import json
import cadquery as cq
import trimesh
import angled_battery_stand as model

OUT=Path(__file__).parent/'output'/'usb-position-only'
model.usb_back=-10.0
model.usb_front=-2.6
model.usb_rearward_mm=3.0

def main():
    p=model.build()
    OUT.mkdir(parents=True,exist_ok=True)
    report={'usb_opening_mm':[model.usb_width,model.usb_front-model.usb_back],
            'rearward_shift_mm':model.usb_rearward_mm,'shift_axis':'local -Z, away from front',
            'note':'Original opening size restored based on printed-fit feedback. Backplate unchanged.'}
    for a,b in [('body','board'),('body','cover'),('body','battery_max')]:
        report[a+'_'+b+'_overlap_mm3']=model.overlap(p[a],p[b])
        print(a,b,report[a+'_'+b+'_overlap_mm3'],flush=True)
    for name in ('body','cover_print'):
        assert p[name].val().isValid() and len(p[name].solids().vals())==1
        path=OUT/(name+'.stl')
        cq.exporters.export(p[name],str(path),tolerance=.01,angularTolerance=.1)
        mesh=trimesh.load(path,force='mesh')
        assert mesh.is_watertight and mesh.is_volume
    cq.exporters.export(p['body'],str(OUT/'body.step'))
    (OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    import os,sys,numpy as np
    os.environ['PYOPENGL_PLATFORM']='egl';np.infty=np.inf
    sys.path.insert(0,'/home/michi/.codex/skills/parametric-3d-printing')
    import preview
    preview.render_multi_view(preview.load_mesh(str(OUT/'body.stl')),str(OUT/'body_views.png'),title='Original USB size, shifted 3 mm rearward')
    print(report,flush=True)

if __name__=='__main__':main()
