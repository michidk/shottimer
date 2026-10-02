"""CAD previews of the inner press-fit rim."""
from PIL import Image,ImageDraw,ImageFont
from render_compact import render,FONT,BOLD
from press_fit_stand import OUT

def main(p):
    import usb_position_only as previous
    before=previous.model.build(False)
    sheet=Image.new('RGB',(1600,980),'#f0f2f4')
    for i,(parts,label) in enumerate(((before,'BEFORE: locking fingers'),(p,'AFTER: inner press-fit rim'))):
        q=dict(parts,body=parts['cover'])
        if i==1:
            q['body']=parts['cover'].cut(parts['rim'])
            q['battery']=parts['rim']
        names=['body'] if i==0 else ['body','battery']
        view=render(q,names,(800,780),azimuth=-65,elevation=23,center=(0,20,36),scale=40)
        sheet.paste(view,(i*800,100))
        ImageDraw.Draw(sheet).text((i*800+35,40),label,font=ImageFont.truetype(BOLD,28),fill='#253038')
    d=ImageDraw.Draw(sheet)
    d.text((40,900),'Blue: inner rim, 3.5 mm deep, with clearance breaks for the battery.',font=ImageFont.truetype(FONT,24),fill='#253038')
    d.text((40,940),'Four shallow contact ribs provide the press fit. No locking fingers or catch recesses.',font=ImageFont.truetype(FONT,22),fill='#253038')
    sheet.save(OUT/'before_after.png')
    import sys,numpy as np
    np.infty=np.inf
    sys.path.insert(0,'/home/michi/.codex/skills/parametric-3d-printing')
    import preview
    for n in ('body','cover_print'):
        preview.render_multi_view(preview.load_mesh(str(OUT/(n+'.stl'))),str(OUT/(n+'_views.png')),title=n.replace('_',' ').title())
