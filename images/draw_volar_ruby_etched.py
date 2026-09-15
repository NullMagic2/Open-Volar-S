"""Open Volar S — Windows 11 inspired monitor color variations.
All shapes, material gradients and shadows are drawn with Python.
Dependencies: pip install pillow numpy
Run this file to recreate the PNGs, Windows ICO files, and concept board.
"""
from pathlib import Path
import zipfile
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont, ImageColor
OUT=Path(__file__).resolve().parent
N=2048
K=N/1024

def rounded_polygon(points,r=20):
    pts=[]
    for i,p in enumerate(points):
        before=np.array(points[i-1],dtype=float)
        here=np.array(p,dtype=float)
        after=np.array(points[(i+1)%len(points)],dtype=float)
        a=here+(before-here)*min(r/np.linalg.norm(before-here),.25)
        b=here+(after-here)*min(r/np.linalg.norm(after-here),.25)
        for t in np.linspace(0,1,12):
            q=(1-t)**2*a+2*(1-t)*t*here+t*t*b
            pts.append(tuple(q))
    return pts

class Painter:
    def __init__(self):
        self.im=Image.new('RGBA',(N,N))
    def mask(self,shape,coords,r=0):
        m=Image.new('L',(N,N))
        d=ImageDraw.Draw(m)
        if shape=='rect':
            d.rounded_rectangle(tuple(round(v*K) for v in coords),radius=round(r*K),fill=255)
        elif shape=='ellipse':
            d.ellipse(tuple(round(v*K) for v in coords),fill=255)
        else:
            pts=rounded_polygon(coords,r) if r else coords
            d.polygon([(round(x*K),round(y*K)) for x,y in pts],fill=255)
        return m
    def fill(self,m,a,b=None,alpha=255):
        box=m.getbbox()
        if not box:return
        x,y,x2,y2=box
        w,h=x2-x,y2-y
        aa=np.array(ImageColor.getrgb(a),dtype=float)
        bb=np.array(ImageColor.getrgb(b or a),dtype=float)
        yy,xx=np.mgrid[0:h,0:w]
        t=np.clip(.70*yy/max(h-1,1)+.30*xx/max(w-1,1),0,1)[...,None]
        rgb=(aa*(1-t)+bb*t).astype('uint8')
        layer=Image.fromarray(rgb).convert('RGBA')
        mask=m.crop(box)
        if alpha!=255: mask=mask.point(lambda v:round(v*alpha/255))
        layer.putalpha(mask)
        self.im.alpha_composite(layer,(x,y))
    def shape(self,kind,coords,r,a,b=None,alpha=255):
        m=self.mask(kind,coords,r)
        self.fill(m,a,b,alpha)
        return m
    def shadow(self,m,blur=16,dy=15,opacity=70):
        m=m.filter(ImageFilter.GaussianBlur(blur*K)).point(lambda v:round(v*opacity/255))
        layer=Image.new('RGBA',(N,N),'#342538')
        layer.putalpha(m)
        self.im.alpha_composite(layer,(0,round(dy*K)))
    def stroke(self,pts,fill,width=3):
        d=ImageDraw.Draw(self.im)
        d.line([(int(x*K),int(y*K)) for x,y in pts],fill=fill,width=max(1,int(width*K)),joint='curve')


def font(size,bold=False):
    for f in [Path('C:/Windows/Fonts')/('segoeuib.ttf' if bold else 'segoeui.ttf'),
              Path('/usr/share/fonts/truetype/dejavu')/('DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf')]:
        if f.exists():return ImageFont.truetype(str(f),size)
    return ImageFont.load_default(size=size)



def curve(a,b,c,d,steps=28):
    a,b,c,d=[np.array(q,dtype=float) for q in (a,b,c,d)]
    return [tuple((1-t)**3*a+3*(1-t)**2*t*b+3*(1-t)*t*t*c+t**3*d) for t in np.linspace(0,1,steps)]

def shackle_points(style):
    if style in ('round','swung'):
        pts=[(414,497),(414,384)]
        pts += [(489+75*np.cos(t),384+75*np.sin(t)) for t in np.linspace(np.pi,2*np.pi,80)]
        pts += [(564,410)]
    else:
        pts=[(414,497),(414,365)]
        pts+=curve((414,365),(414,339),(434,320),(460,320))
        pts += [(518,320)]
        pts+=curve((518,320),(544,320),(564,339),(564,365))
        pts += [(564,410)]
    if style=='swung':
        origin=np.array((414,492),dtype=float)
        a=np.deg2rad(-19)
        rot=np.array([[np.cos(a),-np.sin(a)],[np.sin(a),np.cos(a)]])
        pts=[tuple(origin+rot@(np.array(q)-origin)) for q in pts]
    return pts

def tube_mask(points,width):
    m=Image.new('L',(N,N))
    d=ImageDraw.Draw(m)
    d.line([(round(x*K),round(y*K)) for x,y in points],fill=255,width=round(width*K),joint='curve')
    r=width/2
    for x,y in (points[0],points[-1]):
        d.ellipse(tuple(round(v*K) for v in (x-r,y-r,x+r,y+r)),fill=255)
    return m

def add_lock(p,style):
    pts=shackle_points(style)
    metal=tube_mask(pts,42)
    p.shadow(metal,5,5,43)
    p.fill(metal,'#FFFFFF','#AFBED0')
    # Narrow continuous highlight follows the same uniform-width bar.
    highlight=tube_mask([(x-7,y-3) for x,y in pts],7)
    h=np.minimum(np.asarray(highlight),np.asarray(metal))
    p.fill(Image.fromarray(h),'#FFFFFF','#E5EFF8',170)
    # Body hides the anchored end; the free end remains clearly separated.
    body=p.mask('rect',(353,478,625,620),30)
    p.shadow(body,9,7,66)
    p.shape('rect',(356,483,628,627),30,'#BCCBDC','#98AFC8')
    p.shape('rect',(353,478,625,620),30,'#FFFFFF','#DFEAF6')
    p.shape('rect',(370,479,607,486),3,'#FFFFFF','#F9FCFF',180)
    p.shape('ellipse',(474,520,504,550),0,'#294A72','#1E3E66')
    p.shape('rect',(480,541,498,576),7,'#23456D','#1D3D65')



def metal_connector(p,bounds):
    x,y,x2,y2=bounds
    w,h=x2-x,y2-y
    def b(u,v,u2,v2):return (x+u*w,y+v*h,x+u2*w,y+v2*h)
    # A visible lower sidewall gives the silver plug more physical depth.
    p.shape('rect',(x+4,y+9,x2+8,y2+12),8,'#91A4BC','#455C79')
    # A narrow outer bevel encloses a satin metal face.
    p.shape('rect',bounds,8,'#D9E4EF','#667B94')
    mask=p.mask('rect',b(.025,.025,.99,.975),6)
    box=mask.getbbox()
    l,t,r,bt=box
    hh,ww=bt-t,r-l
    yy,xx=np.mgrid[0:hh,0:ww]
    u=xx/max(ww-1,1)
    v=yy/max(hh-1,1)
    stops=np.array([0,.09,.22,.44,.67,.83,1])
    colors=np.array([[132,149,169],[248,252,255],[213,225,237],
                     [147,166,188],[239,247,253],[195,212,231],[177,155,168]])
    # Broad reflected light bands replace the former flat diagonal fill.
    rgb=np.stack([np.interp(u,stops,colors[:,j]) for j in range(3)],axis=-1)
    # Low-amplitude vertical brushing is visible only in closeups.
    brushing=(np.sin(xx*.8)+np.sin(xx*1.7))*.52
    rgb += brushing[...,None]
    surface=Image.fromarray(np.clip(rgb,0,255).astype('uint8')).convert('RGBA')
    surface.putalpha(mask.crop(box))
    p.im.alpha_composite(surface,(l,t))
    # Leading opening, a bright lip, and a shaded lower edge.
    p.shape('rect',b(.018,.075,.076,.925),3,'#8396AC','#40546E')
    p.shape('rect',b(.077,.055,.097,.945),2,'#F9FDFF','#D8E4EF')
    p.shape('rect',b(.09,.028,.965,.044),1,'#FFFFFF','#F1F7FC',210)
    p.shape('rect',b(.09,.952,.967,.973),1,'#849AB3','#59738F',190)
    for top in (.19,.64):
        hole=b(.40,top,.595,top+.18)
        p.shape('rect',hole,4,'#172D47','#49617A')
        p.shape('rect',b(.42,top+.025,.575,top+.063),2,'#102239','#1D334D')
        p.shape('rect',b(.42,top+.165,.575,top+.180),1,'#AFC2D4','#7891AA')


def ruby_tuner():
    p=Painter()
    shell=p.mask('rect',(325,379,878,605),44)
    p.shadow(shell,16,16,78)
    metal_connector(p,(186,415,341,570))
    p.shape('rect',(308,407,363,580),22,'#B9435A','#4C0C25')
    # A continuous molded sidewall, 22 units deep, with matching corner radii.
    # The previous stepped, oversized back slab is replaced by one extrusion.
    side=Image.new('L',(N,N))
    sd=ImageDraw.Draw(side)
    for t in np.linspace(0,1,24):
        box=(325+6*t,379+22*t,878+6*t,605+22*t)
        sd.rounded_rectangle(tuple(round(v*K) for v in box),radius=round(44*K),fill=255)
    p.fill(side,'#991A39','#360719')
    # Ruby-tinted shell: a lit perimeter surrounds the darker inner cavity.
    p.shape('rect',(325,379,878,605),44,'#E26479','#78122F')
    p.shape('rect',(331,385,872,598),39,'#BC2344','#520B23')
    p.shape('rect',(371,419,831,561),24,'#6D0B29','#35071A',133)
    # Thin highlights and light through the lower edge suggest translucent resin.
    p.shape('rect',(350,394,848,410),8,'#FFE0DE','#BD5B70',170)
    p.shape('rect',(369,576,834,583),4,'#DB607A','#8B213E',125)
    p.shape('poly',[(343,424),(775,413),(665,461),(341,474)],10,'#FFB0BC','#CF516D',37)
    painted_padlock(p)
    return p.im



def painted_padlock(p):
    # Restore the unbent source artwork: a semicircular shackle and straight
    # body edges. Only this marking receives the perspective transform.
    center=(606,474)
    factor=.43
    def pt(q):
        return (center[0]+(q[0]-489)*factor,center[1]+(q[1]-455)*factor)
    mark=tube_mask([pt(q) for q in shackle_points('round')],42*factor)
    draw=ImageDraw.Draw(mark)
    a,b=pt((319,478)),pt((659,656))
    draw.rounded_rectangle(tuple(round(v*K) for v in (*a,*b)),radius=round(30*factor*K),fill=255)
    a,b=pt((474,538)),pt((504,568))
    draw.ellipse(tuple(round(v*K) for v in (*a,*b)),fill=0)
    a,b=pt((480,559)),pt((498,594))
    draw.rounded_rectangle(tuple(round(v*K) for v in (*a,*b)),radius=round(7*factor*K),fill=0)
    mark=perspective_lock(mark)
    p.fill(mark,'#FAF5F3',alpha=250)
    # Retain only the shallow white-filled etch, without geometric bending.
    step=max(1,round(1.15*K))
    down=Image.new('L',(N,N))
    down.paste(mark,(step,step))
    up=Image.new('L',(N,N))
    up.paste(mark,(-step,-step))
    arr=np.asarray(mark,dtype=np.int16)
    edge_top=Image.fromarray(np.clip(arr-np.asarray(down,dtype=np.int16),0,255).astype('uint8'))
    edge_bottom=Image.fromarray(np.clip(arr-np.asarray(up,dtype=np.int16),0,255).astype('uint8'))
    p.fill(edge_top,'#743447',alpha=87)
    p.fill(edge_bottom,'#FFFFFF',alpha=170)


def perspective_lock(mark):
    # Perspective applies only to the lock alpha mask, never to the hardware.
    # The upper edge recedes slightly; horizontal edges follow the casing.
    x0,y0,x1,y1=mark.getbbox()
    width=x1-x0
    height=y1-y0
    src=np.array([(x0,y0),(x1,y0),(x1,y1),(x0,y1)],dtype=float)
    dst=np.array([(x0+.045*width,y0+.045*height),
                  (x1-.045*width,y0+.045*height),
                  (x1+.01*width,y1-.045*height),
                  (x0-.01*width,y1-.045*height)],dtype=float)
    rows=[]; values=[]
    for (x,y),(u,v) in zip(dst,src):
        rows.extend([[x,y,1,0,0,0,-u*x,-u*y],
                     [0,0,0,x,y,1,-v*x,-v*y]])
        values.extend([u,v])
    coefficients=np.linalg.solve(np.array(rows),np.array(values))
    return mark.transform((N,N),Image.Transform.PERSPECTIVE,
                          tuple(coefficients),resample=Image.Resampling.BICUBIC)

def render_painted(angle):
    raw=ruby_tuner().rotate(angle,resample=Image.Resampling.BICUBIC,center=(530*K,500*K))
    # Size from solid hardware, not the diffuse shadow. Keep its proportions,
    # with 12 px of room at either side of a 1024 px Windows icon canvas.
    box=raw.getchannel('A').point(lambda v:255 if v>=128 else 0).getbbox()
    x,y,x2,y2=box
    factor=1000/max(x2-x,y2-y)
    left=(1024-(x2-x)*factor)/2
    top=(1024-(y2-y)*factor)/2
    # Transform the entire raster so soft shadows survive beyond the core bounds.
    enlarged=raw.transform((2048,2048),Image.Transform.AFFINE,
        (1/(2*factor),0,x-left/factor,0,1/(2*factor),y-top/factor),
        resample=Image.Resampling.BICUBIC)
    return enlarged.resize((1024,1024),Image.Resampling.LANCZOS)

def main():
    board=Image.new('RGB',(1660,1160),'#F5F7FB')
    d=ImageDraw.Draw(board)
    def text(x,y,t,size=24,color='#19324E',bold=False):
        d.text((x,y),t,font=font(size,bold),fill=color)
    text(60,37,'OPEN VOLAR S / PERSPECTIVE STUDY',21,'#68798C',True)
    text(60,85,'Perspective on the marking.',55,'#19324E',True)
    text(60,163,'Only the white lock is perspective-mapped onto the original ruby casing.',23,'#68798C')
    for i,(angle,name) in enumerate([(12,'Gentle angle'),(30,'Steeper angle')]):
        x=60+i*800
        text(x,243,f'{i+1:02}  {name}',31,'#19324E',True)
        d.rounded_rectangle((x,307,x+740,897),radius=28,fill='#E8EEF6')
        im=render_painted(angle)
        hero=im.resize((710,710),Image.Resampling.LANCZOS)
        board.paste(hero,(x+15,245),hero)
        text(x,936,'NATIVE PIXEL SIZES',16,'#68798C',True)
        for dx,s in [(0,24),(94,32),(198,48),(310,64)]:
            small=im.resize((s,s),Image.Resampling.LANCZOS)
            board.paste(small,(x+dx,978+(64-s)//2),small)
            text(x+dx,1060,str(s)+'px',16,'#68798C')
        d.rounded_rectangle((x+488,931,x+740,1090),radius=18,fill='#14233C')
        dark=im.resize((166,166),Image.Resampling.LANCZOS)
        board.paste(dark,(x+531,928),dark)
        for s in (16,20,24,32,40,48,64,96,128,256,512,1024):
            im.resize((s,s),Image.Resampling.LANCZOS).save(OUT/f'volar-ruby-etched-{i+1}-{s}.png')
        im.save(OUT/f'volar-ruby-etched-{i+1}.ico',sizes=[(s,s) for s in (16,20,24,32,40,48,64,128,256)])
    text(60,1122,'PYTHON-DRAWN / Perspective-mapped white etch / Transparent PNGs and Windows ICOs',17,'#68798C')
    board.save(OUT/'volar-ruby-etched-lock-concepts.png')
    # An unrotated detail makes the vertical metal finish easy to inspect.
    raw=ruby_tuner()
    crop=raw.crop(tuple(int(v*K) for v in (172,397,443,607)))
    crop=crop.resize((813,630),Image.Resampling.LANCZOS)
    detail=Image.new('RGB',(940,760),'#E8EEF6')
    dd=ImageDraw.Draw(detail)
    dd.text((48,32),'VERTICAL SATIN METAL / RUBY CASING',font=font(22,True),fill='#19324E')
    detail.paste(crop,(60,91),crop)
    detail.save(OUT/'volar-ruby-etched-metal-detail.png')

    # Enlarged material sample for review of the white-filled engraving.
    raw=ruby_tuner()
    close=raw.crop(tuple(int(v*K) for v in (521,399,703,582)))
    close=close.resize((650,654),Image.Resampling.LANCZOS)
    sample=Image.new('RGB',(770,774),'#E8EEF6')
    sd=ImageDraw.Draw(sample)
    sd.text((40,24),'PERSPECTIVE / WHITE ETCH',font=font(22,True),fill='#19324E')
    sample.paste(close,(60,85),close)
    sample.save(OUT/'volar-ruby-etched-symbol-detail.png')

    with zipfile.ZipFile(OUT/'volar-ruby-etched-lock-icons.zip','w',zipfile.ZIP_DEFLATED) as z:
        for f in list(OUT.glob('volar-ruby-etched-*.png'))+list(OUT.glob('volar-ruby-etched-*.ico'))+[Path(__file__)]:
            z.write(f,f.name)
    for i in (1,2):
        with Image.open(OUT/f'volar-ruby-etched-{i}.ico') as ico:
            assert (16,16) in ico.ico.sizes() and (256,256) in ico.ico.sizes()
    print('Created and verified the ruby-red icons, vertical metal detail and downloadable source archive.')

if __name__=='__main__':main()
