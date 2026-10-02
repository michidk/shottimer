"""Feature preview of rear-cover retention and integral floor."""
from PIL import Image,ImageDraw,ImageFont
from render_compact import render,FONT,BOLD
from angled_battery_stand import OUT

def main(p,report):
    # Reuse renderer's bezel color for the rear-cover part.
    p=dict(p,bezel=p['cover'])
    sheet=Image.new('RGB',(1600,1100),'#f0f2f4')
    d=ImageDraw.Draw(sheet)
    def text(x,y,t,size=22,bold=False): d.text((x,y),t,font=ImageFont.truetype(BOLD if bold else FONT,size),fill='#253038')
    text(45,25,'1 mm taller. Angled battery and support wedge.',36,True)
    text(45,80,'Battery leans back 10° into an integral support wedge; extended lower pocket.',23)
    sheet.paste(render(p,['body','bezel','board','active'],(800,650),scale=44),(0,120))
    exploded=dict(p)
    exploded['bezel']=p['cover'].translate((0,32,0))
    sheet.paste(render(exploded,['body','bezel'],(800,650),azimuth=55,elevation=25,center=(0,20,35),scale=53),(800,120))
    d=ImageDraw.Draw(sheet)
    text(50,800,'INTEGRATED BODY',24,True)
    text(50,843,'Two lower PCB rim pockets; low USB-tab stops.',21)
    text(50,878,'Tilt the board under the lips, then seat it.',21)
    text(830,800,'REAR COVER + THREE RETAINERS',24,True)
    text(830,843,'Extended angled pocket and a full-width return lip.',21)
    text(830,878,'52 mm length allowance; 0.4 mm clearance per side.',21)
    text(50,975,'Both pieces revised — smooth side walls; pull-release cover with an underside fingernail notch.',20)
    text(50,1015,'Print body upright. Print cover with its outer face on the bed and retainers pointing upward.',20)
    sheet.save(OUT/'preview.png')
    section=render(p,['body','bezel','board','battery'],(1100,850),azimuth=0,elevation=0,center=(0,5,35),scale=42,section=True)
    section.save(OUT/'cutaway.png')
    # Cover alone, viewing its inside and all three supports.
    cover=render(dict(p,body=p['cover']),['body'],(1100,850),azimuth=-60,elevation=20,center=(0,20,36),scale=44)
    cover.save(OUT/'cover_detail.png')
    detail=render(p,['body'],(1100,850),azimuth=95,elevation=22,center=(0,6,49),scale=31)
    detail.save(OUT/'front_pockets.png')
    sequence=Image.new('RGB',(1560,720),'#f0f2f4')
    labels=['1. Tilt back; start slightly high','2. Slide the lower rim under the lips','3. Pivot into the locating guides']
    for i,(angle,shift,travel) in enumerate(((6,1.2,20),(6,0,0),(0,0,0))):
        q=dict(p)
        moved=p['board_local'].rotate((0,-18.25,-4.8),(1,-18.25,-4.8),-angle).translate((0,shift,-travel))
        q['board']=p['pose'](moved)
        sequence.paste(render(q,['body','board'],(520,600),azimuth=75,elevation=25,center=(0,10,48),scale=38),(i*520,70))
    sd=ImageDraw.Draw(sequence)
    sd.text((25,15),'Board insertion into the lower rim pockets',font=ImageFont.truetype(BOLD,28),fill='#253038')
    for i,label in enumerate(labels):
        sd.text((i*520+15,677),label,font=ImageFont.truetype(FONT,20),fill='#253038')
    sequence.save(OUT/'board_insertion.png')

