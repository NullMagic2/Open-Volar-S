"""Export editable vector paths from the approved Python shape construction.
Requires Pillow and NumPy. Run beside draw_volar_ruby_etched.py.
The metal's fine brushing is represented by sampled vertical gradient stops.
"""
from pathlib import Path
import importlib.util
import numpy as np
from PIL import Image, ImageColor
HERE = Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('ruby',HERE/'draw_volar_ruby_etched.py')
g=importlib.util.module_from_spec(spec);spec.loader.exec_module(g)

def simplify(points, eps=.7):
    a=np.asarray(points,dtype=float)
    if len(a)<3:return a.tolist()
    vec=a[-1]-a[0]; length=float(vec@vec)
    if length:
        t=np.clip(((a-a[0])@vec)/length,0,1)
        dist=np.sum((a-(a[0]+t[:,None]*vec))**2,axis=1)
    else:dist=np.sum((a-a[0])**2,axis=1)
    i=int(np.argmax(dist))
    if dist[i]<=eps*eps:return [a[0].tolist(),a[-1].tolist()]
    return simplify(a[:i+1],eps)[:-1]+simplify(a[i:],eps)

def path(mask, offset=(0,0)):
    # Trace oriented pixel boundaries, including the keyhole as a separate contour.
    a=np.pad(np.asarray(mask)>127,1)
    edges={}
    def add(start,end):edges.setdefault(start,[]).append(end)
    yy,xx=np.where(a[1:-1,1:-1]&~a[:-2,1:-1])
    for y,x in zip(yy.tolist(),xx.tolist()):add((x,y),(x+1,y))
    yy,xx=np.where(a[1:-1,1:-1]&~a[1:-1,2:])
    for y,x in zip(yy.tolist(),xx.tolist()):add((x+1,y),(x+1,y+1))
    yy,xx=np.where(a[1:-1,1:-1]&~a[2:,1:-1])
    for y,x in zip(yy.tolist(),xx.tolist()):add((x+1,y+1),(x,y+1))
    yy,xx=np.where(a[1:-1,1:-1]&~a[1:-1,:-2])
    for y,x in zip(yy.tolist(),xx.tolist()):add((x,y+1),(x,y))
    contours=[]
    while edges:
        start=next(iter(edges));p=start;points=[p]
        while True:
            ns=edges[p];n=ns.pop()
            if not ns:del edges[p]
            points.append(n);p=n
            if p==start:break
        pts=simplify(points)
        if len(pts)<4:continue
        contours.append('M'+' L'.join(f'{x+offset[0]:.2f},{y+offset[1]:.2f}' for x,y in pts[:-1])+' Z')
    return ' '.join(contours)

layers=[];defs=[];busy=False
original_fill=g.Painter.fill;original_shadow=g.Painter.shadow;original_composite=Image.Image.alpha_composite

def fill(self,m,a,b=None,alpha=255):
    global busy
    d=path(m)
    if d:
        name='gradient'+str(len(defs));x,y,x2,y2=m.getbbox();w=x2-x;h=y2-y
        aa=.3/max(w-1,1);bb=.7/max(h-1,1);den=aa*aa+bb*bb
        defs.append(f'<linearGradient id="{name}" gradientUnits="userSpaceOnUse" x1="{x}" y1="{y}" x2="{x+aa/den}" y2="{y+bb/den}"><stop stop-color="{a}"/><stop offset="1" stop-color="{b or a}"/></linearGradient>')
        layers.append(f'<path d="{d}" fill="url(#{name})" opacity="{alpha/255:.6f}" fill-rule="evenodd"/>')
    busy=True; original_fill(self,m,a,b,alpha);busy=False

def shadow(self,m,blur=16,dy=15,opacity=70):
    global busy
    name='shadow'+str(len(defs))
    defs.append(f'<filter id="{name}" x="-50%" y="-100%" width="200%" height="300%" color-interpolation-filters="sRGB"><feGaussianBlur stdDeviation="{blur*g.K}"/></filter>')
    layers.append(f'<path d="{path(m)}" transform="translate(0 {dy*g.K})" fill="#342538" opacity="{opacity/255}" filter="url(#{name})"/>')
    busy=True;original_shadow(self,m,blur,dy,opacity);busy=False

def composite(self,im,dest=(0,0),source=(0,0)):
    if not busy:
        # The connector's custom satin-metal surface is the only direct composite.
        d=path(im.getchannel('A'),dest);name='metal'+str(len(defs));a=np.asarray(im);row=a[a.shape[0]//2,:,:3]
        stops=''.join(f'<stop offset="{i/(len(row)-1):.6f}" stop-color="#{row[i,0]:02x}{row[i,1]:02x}{row[i,2]:02x}"/>' for i in sorted(set(np.linspace(0,len(row)-1,96).astype(int))))
        defs.append(f'<linearGradient id="{name}" gradientUnits="userSpaceOnUse" x1="{dest[0]}" y1="0" x2="{dest[0]+im.width-1}" y2="0">{stops}</linearGradient>')
        layers.append(f'<path d="{d}" fill="url(#{name})"/>')
    return original_composite(self,im,dest,source)

g.Painter.fill=fill;g.Painter.shadow=shadow;Image.Image.alpha_composite=composite
raw=g.ruby_tuner()
g.Painter.fill=original_fill;g.Painter.shadow=original_shadow;Image.Image.alpha_composite=original_composite
for angle,name in [(12,'open-volar-s'),(30,'open-volar-s-steep')]:
    rotated=raw.rotate(angle,resample=Image.Resampling.BICUBIC,center=(530*g.K,500*g.K));x,y,x2,y2=rotated.getchannel('A').point(lambda v:255 if v>=128 else 0).getbbox()
    w,h=x2-x,y2-y;factor=1000/max(w,h);ow,oh=w*factor,h*factor
    transform=f'translate({(1024-ow)/2} {(1024-oh)/2}) scale({ow/w} {oh/h}) translate({-x} {-y}) rotate({-angle} {530*g.K} {500*g.K})'
    svg=f'''<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024" role="img" aria-labelledby="title desc">
<title id="title">Open Volar S — unlocked ruby tuner</title>
<desc id="desc">Translucent red USB TV tuner with vertical satin metal reflections and a white etched open padlock mapped to its surface. Python-generated editable vector paths.</desc>
<defs>{''.join(defs)}</defs><g transform="{transform}">{''.join(layers)}</g></svg>'''
    (HERE/(name+'.svg')).write_text(svg,encoding='utf8')
    print(name+'.svg',len(svg),'bytes',len(layers),'vector layers')
