"""Manufacturer reference with the measured PCB/display stack correction."""
from pathlib import Path
import cadquery as cq

ROOT = Path(__file__).resolve().parents[1]
board_glass_front_z = -0.4
board_source_center = (18.01, -0.142, 0.0)
source_glass_front_z = -2.0
MEASURED_PCB_THICKNESS = 1.6
MEASURED_DISPLAY_THICKNESS = 2.3
# Raw STEP bounding-box sizes (x, y, z) that identify required components.
# Solid order in a STEP compound is not a contract, so roles are resolved by
# geometry and must match exactly one solid.
COMPONENT_SIZES = {
    'pcb': (39.512, 36.523, 0.411),
    'display': (35.6, 35.6, 1.0),
    'usb_shell': (7.53, 9.58, 4.16),
}
COMPONENT_SIZE_TOLERANCE = 0.05
_component_index = {}

def _resolve_components(solids):
    for name,size in COMPONENT_SIZES.items():
        matches=[i for i,solid in enumerate(solids)
                 if all(abs(a-b)<=COMPONENT_SIZE_TOLERANCE for a,b in zip(
                     (solid.BoundingBox().xlen,solid.BoundingBox().ylen,solid.BoundingBox().zlen),size))]
        assert len(matches)==1,('reference component',name,matches)
        _component_index[name]=matches[0]

def board_part(board, name):
    """Return a named reference component from a board built by import_board."""
    return board.solids().vals()[_component_index[name]]

def import_board(include_display_connector=True, correct_stack=True):
    """Import manufacturer geometry plus the user-confirmed display connector envelope."""
    board = (cq.importers.importStep(str(ROOT/'reference'/'RP2040-LCD-1.28.step'))
            .translate(tuple(-v for v in board_source_center))
            .rotate((0,0,0),(1,1,0),180)
            .translate((0,0,board_glass_front_z+source_glass_front_z)))
    _resolve_components(board.solids().vals())
    pcb_index,display_index=_component_index['pcb'],_component_index['display']
    if correct_stack:
        # User-measured stack, superseding the simplified STEP thicknesses.
        # Keep the component-side PCB plane fixed; rear components retain position.
        solids=board.solids().vals()
        pcb_box=solids[pcb_index].BoundingBox()
        pcb_back=pcb_box.zmin
        display_back=pcb_back+MEASURED_PCB_THICKNESS
        def resize_z(solid,thickness,new_bottom):
            bounds=solid.BoundingBox()
            factor=thickness/bounds.zlen
            matrix=cq.Matrix([[1,0,0,0],[0,1,0,0],
                              [0,0,factor,new_bottom-factor*bounds.zmin],[0,0,0,1]])
            result=solid.transformGeometry(matrix)
            assert result.isValid()
            return result
        solids[pcb_index]=resize_z(solids[pcb_index],MEASURED_PCB_THICKNESS,pcb_back)
        solids[display_index]=resize_z(solids[display_index],MEASURED_DISPLAY_THICKNESS,display_back)
        board=cq.Workplane('XY').newObject([cq.Compound.makeCompound(solids)])
    if not include_display_connector:
        return board
    # User's physical module has a display connector over the USB-side tab,
    # reaching the LCD front plane. STEP omits it. Reserve the full tab footprint;
    # this is a conservative clearance envelope, not a measured connector shape.
    pcb=board_part(board,'pcb')
    lcd=board_part(board,'display')
    bottom=pcb.BoundingBox().zmax
    top=lcd.BoundingBox().zmax
    x0,x1=15.5,21.262
    y1=6.404
    y0=y1+(x1-x0)*(9.187-6.404)/(21.262-15.781)
    connector=(cq.Workplane('XY').workplane(offset=bottom)
               .polyline([(x0,-y0),(x1,-y1),(x1,y1),(x0,y0)])
               .close().extrude(top-bottom))
    connector=connector.cut(cq.Workplane('XY').newObject([lcd]))
    return cq.Workplane('XY').newObject([cq.Compound.makeCompound(
        board.solids().vals()+connector.solids().vals())])
