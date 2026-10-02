"""Feature preview of rear-cover retention and integral floor."""
from PIL import Image,ImageDraw,ImageFont
from render_compact import render,FONT,BOLD
from screwless_stand import OUT

def main(p,report):
    # Reuse renderer's bezel color for the rear-cover part.
    p=dict(p,bezel=p['cover'])
    sheet=Image.new('RGB',(1600,1100),'#f0f2f4')
    d=ImageDraw.Draw(sheet)
    def text(x,y,t,size=22,bold=False): d.text((x,y),t,font=ImageFont.truetype(BOLD if bold else FONT,size),fill='#253038')
    text(45,25,'Screwless rear cover. Press the sides to release.',36,True)
    text(45,80,'Flat bottom on the bed; no bottom chamfer or separate front ring.',23)
    sheet.paste(render(p,['body','bezel','board','active'],(800,650),scale=44),(0,120))
    exploded=dict(p)
    exploded['bezel']=p['cover'].translate((0,32,0))
    sheet.paste(render(exploded,['body','bezel'],(800,650),azimuth=55,elevation=25,center=(0,20,35),scale=53),(800,120))
    d=ImageDraw.Draw(sheet)
    text(50,800,'INTEGRATED BODY',24,True)
    text(50,843,'The front lip seats the display; the floor is fixed.',21)
    text(50,878,'Rear-load screen, then cover with battery attached.',21)
    text(830,800,'REAR COVER + THREE RETAINERS',24,True)
    text(830,843,'Side arms pass around the upright battery.',21)
    text(830,878,'Two side-release clips; soft pads at the PCB edges.',21)
    text(50,975,'Feature prototype — test the PLA snap coupon before printing the enclosure.',20)
    text(50,1015,'Print body upright. Print cover with its outer face on the bed and retainers pointing upward.',20)
    sheet.save(OUT/'preview.png')
    section=render(p,['body','bezel','board','battery'],(1100,850),azimuth=0,elevation=0,center=(0,5,35),scale=42,section=True)
    section.save(OUT/'cutaway.png')
    # Cover alone, viewing its inside and all three supports.
    cover=render(dict(p,body=p['cover']),['body'],(1100,850),azimuth=-60,elevation=20,center=(0,20,36),scale=44)
    cover.save(OUT/'cover_detail.png')
