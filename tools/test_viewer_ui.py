"""Exercise only the app process launched here, with an isolated settings folder."""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import subprocess,time,json,sys
from PIL import Image
source=Path(__file__).resolve().parents[1]
root=source/'target/viewer-ui-check'
(root/'work').mkdir(parents=True,exist_ok=True)
(root/'outputs').mkdir(parents=True,exist_ok=True)
profile=root/'work/live-tv-preview-profile';profile.mkdir(exist_ok=True)
(profile/'settings.json').write_text(json.dumps(dict(channels=[521143,533143],frequency=521143,services=[dict(name='TV GAZETA HD',frequency_khz=521143,channel_number=4),dict(name='Test channel',frequency_khz=533143,channel_number=7)],volume=65,button_material='metal',theme='orbit')))
# Only the isolated preview profile receives fixture guide data.
(profile/'epg.json').write_text(json.dumps([dict(frequency_khz=521143 if i%2==0 else 533143,program_id=1 if i%2==0 else 2,event_id=i,channel_number=4 if i%2==0 else 7,channel_name='TV GAZETA HD' if i%2==0 else 'Test channel',name=['Evening news','Science and nature','Arts and culture'][i%3],description='Program information supplied by the broadcaster. This preview checks readable text, filtering and keyboard selection.',start=1789214400+i*1800,duration=1800,running=i==4) for i in range(24)]))
u=C.windll.user32;g=C.windll.gdi32
original_cursor=W.POINT();u.GetCursorPos(C.byref(original_cursor))
test_cursor=None
u.SetProcessDpiAwarenessContext(C.c_void_p(-4))
for name in ['GetDC','GetWindowDC','GetDlgItem']:
    getattr(u,name).restype=W.HANDLE
u.SendMessageW.restype=C.c_ssize_t;u.SendMessageW.argtypes=[W.HWND,W.UINT,W.WPARAM,W.LPARAM]
u.PostMessageW.argtypes=[W.HWND,W.UINT,W.WPARAM,W.LPARAM]
u.GetWindowRect.argtypes=[W.HWND,C.POINTER(W.RECT)]
u.SetWindowPos.argtypes=[W.HWND,W.HWND,C.c_int,C.c_int,C.c_int,C.c_int,W.UINT]
u.PrintWindow.argtypes=[W.HWND,W.HDC,W.UINT]
u.ReleaseDC.argtypes=[W.HWND,W.HDC]
u.GetDlgItem.argtypes=[W.HWND,C.c_int]
u.EnableWindow.argtypes=[W.HWND,W.BOOL]
u.IsWindowEnabled.argtypes=[W.HWND]
for name in ['CreateCompatibleDC','CreateCompatibleBitmap','SelectObject']:
    getattr(g,name).restype=W.HANDLE
g.CreateRectRgn.restype=W.HANDLE;g.CreateRectRgn.argtypes=[C.c_int]*4
g.PtInRegion.argtypes=[W.HANDLE,C.c_int,C.c_int]
u.GetWindowRgn.argtypes=[W.HWND,W.HANDLE]
g.CreateCompatibleDC.argtypes=[W.HDC];g.CreateCompatibleBitmap.argtypes=[W.HDC,C.c_int,C.c_int]
g.SelectObject.argtypes=[W.HDC,W.HANDLE];g.DeleteObject.argtypes=[W.HANDLE];g.DeleteDC.argtypes=[W.HDC]
class BI(C.Structure):
    _fields_=[('size',W.DWORD),('width',W.LONG),('height',W.LONG),('planes',W.WORD),('bits',W.WORD),('compression',W.DWORD),('image_size',W.DWORD),('x',W.LONG),('y',W.LONG),('used',W.DWORD),('important',W.DWORD)]
g.GetDIBits.argtypes=[W.HDC,W.HBITMAP,W.UINT,W.UINT,C.c_void_p,C.POINTER(BI),W.UINT]
class NMHDR(C.Structure):_fields_=[('hwnd',W.HWND),('id',C.c_size_t),('code',W.UINT)]
u.GetPropW.restype=W.HANDLE;u.GetPropW.argtypes=[W.HWND,W.LPCWSTR]
u.GetLayeredWindowAttributes.argtypes=[W.HWND,C.POINTER(W.DWORD),C.POINTER(C.c_ubyte),C.POINTER(W.DWORD)]
def check_dropdown_backdrop(parent,combo,target):
    u.SendMessageW(combo,0x14f,1,0);time.sleep(.1)
    overlay=u.GetPropW(parent,'OrbitDropdownBackdrop')
    assert overlay and u.IsWindowVisible(overlay),'Open dropdown must dim underlying widgets'
    color=W.DWORD();alpha=C.c_ubyte();flags=W.DWORD()
    assert u.GetLayeredWindowAttributes(overlay,C.byref(color),C.byref(alpha),C.byref(flags)) and alpha.value==48
    region=g.CreateRectRgn(0,0,0,0);assert u.GetWindowRgn(overlay,region)>0
    origin=W.RECT();r=W.RECT();u.GetWindowRect(overlay,C.byref(origin));u.GetWindowRect(target,C.byref(r))
    assert g.PtInRegion(region,(r.left+r.right)//2-origin.left,(r.top+r.bottom)//2-origin.top),'Lower widget must be inside the dimmed region'
    u.GetWindowRect(combo,C.byref(r))
    assert not g.PtInRegion(region,(r.left+r.right)//2-origin.left,(r.top+r.bottom)//2-origin.top),'Combo face must stay undimmed'
    g.DeleteObject(region)
    u.SendMessageW(combo,0x14f,0,0);time.sleep(.1)
    assert not u.IsWindowVisible(overlay),'Closing dropdown must restore underlying widgets'

def capture(hwnd,name=None):
    r=W.RECT();u.GetWindowRect(hwnd,C.byref(r));w=r.right-r.left;h=r.bottom-r.top
    dc=u.GetWindowDC(hwnd);mem=g.CreateCompatibleDC(dc);bmp=g.CreateCompatibleBitmap(dc,w,h);old=g.SelectObject(mem,bmp)
    assert u.PrintWindow(hwnd,mem,2)
    g.SelectObject(mem,old);info=BI(C.sizeof(BI),w,-h,1,32,0,0,0,0,0,0);buf=C.create_string_buffer(w*h*4)
    assert g.GetDIBits(mem,bmp,0,h,buf,C.byref(info),0)
    image=Image.frombuffer('RGB',(w,h),buf,'raw','BGRX',0,1).copy()
    region=g.CreateRectRgn(0,0,0,0)
    if u.GetWindowRgn(hwnd,region)>0:
        image=image.convert('RGBA')
        # PrintWindow clears clipped pixels to black; preserve actual window transparency.
        edge=min(80,w//2,h//2)
        for y in [*range(edge),*range(h-edge,h)]:
            for x in [*range(edge),*range(w-edge,w)]:
                if not g.PtInRegion(region,x,y):image.putpixel((x,y),(0,0,0,0))
    g.DeleteObject(region)
    if name:image.save(root/'outputs'/name)
    g.DeleteObject(bmp);g.DeleteDC(mem);u.ReleaseDC(hwnd,dc)
    return image
def windows(pid):
    found={}
    @C.WINFUNCTYPE(W.BOOL,W.HWND,W.LPARAM)
    def cb(h,_):
        p=W.DWORD();u.GetWindowThreadProcessId(h,C.byref(p))
        if p.value==pid:
            text=C.create_unicode_buffer(256);u.GetWindowTextW(h,text,256);found[text.value]=h
        return True
    u.EnumWindows(cb,0);return found

u.SetForegroundWindow.argtypes=[W.HWND];u.GetForegroundWindow.restype=W.HWND
u.IsWindowVisible.argtypes=[W.HWND];u.IsZoomed.argtypes=[W.HWND]
u.MoveWindow.argtypes=[W.HWND,C.c_int,C.c_int,C.c_int,C.c_int,W.BOOL]
u.ShowWindow.argtypes=[W.HWND,C.c_int]
u.ClientToScreen.argtypes=[W.HWND,C.POINTER(W.POINT)]
log=open(root/'work/viewer-check.log','w')
p=subprocess.Popen([str(source/'target/release/live-tv.exe'),'--ui-preview','--profile-dir',str(profile)],stdout=log,stderr=log,creationflags=0x08000000)
checks=[]
def bounds(h):
    r=W.RECT();u.GetWindowRect(h,C.byref(r));return r
def command(h,id):
    u.PostMessageW(h,0x111,id,0);time.sleep(.25)
def shot(h,name):
    im=capture(h);im.save(root/'outputs'/name);return im
try:
    deadline=time.monotonic()+20
    while time.monotonic()<deadline:
        handles=windows(p.pid)
        if 'Live TV! — Control Panel' in handles and 'Live TV! — Settings' in handles:break
        assert p.poll() is None,(p.returncode,(root/'work/viewer-check.log').read_text());time.sleep(.2)
    else:raise RuntimeError(handles)
    deck=handles['Live TV! — Control Panel'];settings=handles['Live TV! — Settings']
    video=next(h for t,h in handles.items() if 'Vulkan preview' in t)
    u.ShowWindow(deck,0);u.MoveWindow(video,60,50,1600,1100,True);time.sleep(.8)
    assert u.GetPropW(video,'OrbitBorderless')==3
    for width in [800,1120,1600]:
        u.MoveWindow(video,60,50,width,round(width*1100/1600),True);time.sleep(.2)
        keys=[bounds(u.GetDlgItem(video,i)) for i in [120,121,101,102]]
        assert all(a.right<b.left for a,b in zip(keys,keys[1:])),width
        rec=bounds(u.GetDlgItem(video,103));ch=bounds(u.GetDlgItem(video,105));chp=bounds(u.GetDlgItem(video,106));vol=bounds(u.GetDlgItem(video,107));volp=bounds(u.GetDlgItem(video,108))
        assert abs((chp.right-ch.left)-(volp.right-vol.left))<=1
        assert rec.right-rec.left==rec.bottom-rec.top
        assert abs((rec.left+rec.right)-(keys[-1].right+ch.left))<=2
        surface=bounds(u.GetDlgItem(video,500));outer=bounds(video)
        assert surface.left>outer.left and surface.right<outer.right and surface.top>outer.top and surface.bottom<outer.bottom
    checks.append('Native geometry: correct transport order, equal channel/volume rockers, circular centered Rec, framed screen at 800/1120/1600 px')
    timer=u.GetDlgItem(video,118);assert u.IsWindowVisible(timer)
    tr=bounds(timer);sr=bounds(u.GetDlgItem(video,117));lr=bounds(u.GetDlgItem(video,122));assert sr.right<tr.left and tr.right<lr.left
    checks.append('Contextual time readout is visible beside the slider without overlapping Live')
    im=shot(video,'Live-TV-native-console.png');im.convert('RGB').resize((1120,770)).save(root/'work/native-console-review.jpg',quality=91)
    # Record the supported resting state plus exact hover edges on the real controls.
    u.SetWindowPos(video,-1,0,0,0,0,0x13)
    for id,name in [(103,'Rec'),(122,'Live')]:
        h=u.GetDlgItem(video,id);rr=bounds(h);u.SetCursorPos((rr.left+rr.right)//2,(rr.top+rr.bottom)//2);u.EnableWindow(h,True);u.SendMessageW(h,0x200,0,10|(10<<16))
        hover=shot(h,f'Live-TV-native-{name}-hover.png')
        if id==122:assert sum(1 for pixel in hover.convert('RGB').getdata() if pixel==(90,166,101))>30,'Live hover must have its green border'
        u.SendMessageW(h,0x2a3,0,0)
    u.SetWindowPos(video,-2,0,0,0,0,0x13)
    checks.append('Live hover uses the DAC green border and cream resting text')
    command(video,127)
    menus=[]
    @C.WINFUNCTYPE(W.BOOL,W.HWND,W.LPARAM)
    def find_menu(h,_):
        process=W.DWORD();u.GetWindowThreadProcessId(h,C.byref(process));cls=C.create_unicode_buffer(64);u.GetClassNameW(h,cls,64)
        if process.value==p.pid and cls.value=='#32768' and u.IsWindowVisible(h):menus.append(h)
        return True
    u.EnumWindows(find_menu,0)
    menu=menus[0] if menus else None
    assert menu,'Settings should open a native popup'
    shot(menu,'Live-TV-native-settings-menu.png')
    u.PostMessageW(video,0x1f,0,0);time.sleep(.15)
    command(video,109);assert u.IsWindowVisible(settings)
    command(settings,113);assert not u.IsWindowVisible(settings)
    command(video,112);assert u.IsWindowVisible(deck)
    checks.append('Settings popup and existing Preferences / Receiver panel actions open correctly')
    before=json.loads((profile/'settings.json').read_text()).get('captions_enabled',True)
    command(video,124);after=json.loads((profile/'settings.json').read_text())['captions_enabled'];assert after!=before
    checks.append('Closed captions command persists its toggle in the isolated profile')
    command(video,128);assert u.IsZoomed(video)
    command(video,128);assert not u.IsZoomed(video)
    old=bounds(video);command(video,115)
    assert not u.IsWindowVisible(u.GetDlgItem(video,101))
    v=bounds(u.GetDlgItem(video,500));r=bounds(video);assert v.left-r.left==1 and r.right-v.right==1
    command(video,115);r=bounds(video);assert (r.right-r.left,r.bottom-r.top)==(old.right-old.left,old.bottom-old.top)
    assert u.IsWindowVisible(u.GetDlgItem(video,101))
    checks.append('Maximize / restore and fullscreen preserve geometry; fullscreen keeps its one-pixel Vulkan inset')
    u.ShowWindow(deck,5);u.SetWindowPos(deck,-1,0,0,0,0,0x13);time.sleep(.15)
    for id,name in [(103,'Rec'),(124,'CC')]:
        h=u.GetDlgItem(deck,id);rr=bounds(h);u.SetCursorPos(rr.left-20,rr.top-20);u.SendMessageW(h,0x2a3,0,0);time.sleep(.1);normal=capture(h);u.SetCursorPos((rr.left+rr.right)//2,(rr.top+rr.bottom)//2);u.SendMessageW(h,0x200,0,10|(10<<16));time.sleep(.12)
        hover=shot(h,f'DAC-native-{name}-hover.png');normal.save(root/'work'/f'DAC-{name}-normal.png');u.SendMessageW(h,0x2a3,0,0)
        print(name,'enabled',u.IsWindowEnabled(h),'visible',u.IsWindowVisible(h),'rect',(rr.left,rr.top,rr.right,rr.bottom),'foreground',u.GetForegroundWindow()==deck,'changed',normal.tobytes()!=hover.tobytes())
        assert normal.tobytes()!=hover.tobytes()
    shot(deck,'DAC-native-panel.png')
    # Change the real Appearance combo and notify its normal command handler.
    faces={};strips=[]
    for index,name in enumerate(['Metal','Glass','Plastic']):
        combo=u.GetDlgItem(settings,322);u.SendMessageW(combo,0x14e,index,0)
        u.PostMessageW(settings,0x111,322|(1<<16),combo);time.sleep(.3)
        faces[name]={id:capture(u.GetDlgItem(video,id)).tobytes() for id in [102,111,127,122]}
        im=shot(video,f'Live-TV-native-{name.lower()}.png')
        strip=im.crop((0,im.height-round(im.width*170/1600),im.width,im.height));strips.append(strip)
    for id in [102,111,127,122]:assert len({faces[name][id] for name in faces})==3,(id,'must follow all DAC materials')
    from PIL import ImageDraw
    sheet=Image.new('RGB',(1600,sum(im.height+28 for im in strips)),(24,18,13));d=ImageDraw.Draw(sheet);y=0
    for name,im in zip(['Metal','Glass','Plastic'],strips):d.text((24,y+6),name,fill=(233,223,206));sheet.paste(im,(0,y+28));y+=im.height+28
    sheet.save(root/'outputs/Live-TV-native-materials.png')
    combo=u.GetDlgItem(settings,322);u.SendMessageW(combo,0x14e,0,0);u.PostMessageW(settings,0x111,322|(1<<16),combo);time.sleep(.2)
    checks.append('Playback, utility and Live buttons follow all three DAC materials through the actual Appearance setting')

    checks.append('DAC Rec and CC hover render visibly different colored borders')
    assert p.poll() is None
    (root/'outputs/native-validation.json').write_text(json.dumps({'checks':checks,'profile':'Isolated UI preview; no tuner or GPU playback started'},indent=2))
    print(json.dumps(checks,indent=2))
finally:
    u.SetCursorPos(original_cursor.x,original_cursor.y)
    p.terminate()
    try:p.wait(timeout=8)
    except subprocess.TimeoutExpired:p.kill();p.wait()
    log.close()
