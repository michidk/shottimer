"""Add a matching PCB-overhang lip to the central lower front finger."""
from pathlib import Path
import json
import cadquery as cq
import trimesh
import corrected_pcb_stand as model

OUT=Path(__file__).parent/'output'/'three-lower-hooks'
HOOK_ANGLES=(240,270,300)

def build(include_board=True):
    model.front_hook_angles=HOOK_ANGLES
    return model.build(include_board)

def main():
    p=build()
    model.front_hook_angles=(240,300)
    previous=model.build(False)
    model.front_hook_angles=HOOK_ANGLES
    added=p['body'].cut(previous['body'])
    assert added.val().Volume()>0
    assert previous['body'].cut(p['body']).val().Volume()<1e-5
    assert model.overlap(added,p['board'])<1e-5
    assert model.overlap(added,p['cover'])<1e-5
    assert model.overlap(added,p['battery_max'])<1e-5
    assert abs(previous['cover'].val().Volume()-p['cover'].val().Volume())<1e-5
    report={'front_hook_angles_deg':HOOK_ANGLES,'radial_overlap_mm':model.front_hook_overlap,
            'pcb_rear_clearance_mm':model.front_hook_gap,'lip_thickness_mm':model.front_hook_t,
            'scope':'Only the central lower PCB holder changes. Existing corrected-pcb-fit checks remain applicable elsewhere.',
            'insertion_overlap_mm3':{}}
    def check(angle,shift,travel,label):
        b=p['board_local'].rotate((0,model.board_insert_pivot_y,model.pcb_back),(1,model.board_insert_pivot_y,model.pcb_back),-angle)
        b=p['pose'](b.translate((0,shift,-travel)))
        v=model.overlap(added,b)
        assert v<1e-5,(label,v)
        report['insertion_overlap_mm3'][label]=round(v,8)
    for travel in model.travel_samples:
        check(12,1.2,travel,'approach_'+str(travel))
    for angle,shift in ((12,1.2),(10,.9),(8,.6),(6,.3),(4,.15),(2,0),(0,0)):
        check(angle,shift,0,'seat_'+str(angle))
    print('Additional lower lip: clearance and insertion passed',flush=True)
    OUT.mkdir(exist_ok=True,parents=True)
    for name in ('body','cover_print'):
        assert p[name].val().isValid() and len(p[name].solids().vals())==1
        path=OUT/(name+'.stl')
        cq.exporters.export(p[name],str(path),tolerance=.01,angularTolerance=.1)
        mesh=trimesh.load(path,force='mesh');assert mesh.is_watertight and mesh.is_volume
    cq.exporters.export(p['body'],str(OUT/'body.step'))
    (OUT/'fit-check.json').write_text(json.dumps(report,indent=2)+'\n')
    from render_compact import render
    # View from open rear, highlighting only the material added to the central finger.
    q=dict(p,battery=added)
    q['body']=p['body'].cut(added)
    render(q,['body','battery'],(1200,950),azimuth=90,elevation=30,center=(0,4,39),scale=32).save(OUT/'lower_hooks.png')
    import sys,numpy as np
    np.infty=np.inf
    sys.path.insert(0,'/home/michi/.codex/skills/parametric-3d-printing')
    import preview
    preview.render_multi_view(preview.load_mesh(str(OUT/'body.stl')),str(OUT/'body_views.png'),title='Three lower PCB retaining lips')

if __name__=='__main__':main()
