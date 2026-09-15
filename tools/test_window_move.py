"""Exercise only the app process launched here, with an isolated settings folder."""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import subprocess,time,json,sys
from PIL import Image
source=Path(__file__).resolve().parents[1]
root=source/'target/move-ui-check'
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

import os
u.IsWindowVisible.argtypes=[W.HWND];u.ClientToScreen.argtypes=[W.HWND,C.POINTER(W.POINT)]
u.ShowWindow.argtypes=[W.HWND,C.c_int];u.GetClientRect.argtypes=[W.HWND,C.POINTER(W.RECT)]
u.SetWindowTextW.argtypes=[W.HWND,W.LPCWSTR];u.KillTimer.argtypes=[W.HWND,C.c_size_t];u.UpdateWindow.argtypes=[W.HWND]
u.GetWindowLongPtrW.restype=C.c_ssize_t;u.GetWindowLongPtrW.argtypes=[W.HWND,C.c_int]
g.BitBlt.argtypes=[W.HDC,C.c_int,C.c_int,C.c_int,C.c_int,W.HDC,C.c_int,C.c_int,W.DWORD]
dw=C.windll.dwmapi;dw.DwmGetWindowAttribute.argtypes=[W.HWND,W.DWORD,C.c_void_p,W.DWORD]
log=open(root/'work/move-preview.log','w');tag=os.environ.get('MOVE_CHECK_TAG','after')
p=subprocess.Popen([os.environ.get('MOVE_CHECK_EXE',str(source/'target/release/live-tv.exe')),'--ui-preview','--profile-dir',str(profile)],stdout=log,stderr=log,creationflags=0x08000000)
def rect(h):
    r=W.RECT();u.GetWindowRect(h,C.byref(r));return r
def screen_capture(h):
    r=rect(h);w=r.right-r.left;hgt=r.bottom-r.top
    dc=u.GetDC(None);mem=g.CreateCompatibleDC(dc);bmp=g.CreateCompatibleBitmap(dc,w,hgt);old=g.SelectObject(mem,bmp)
    assert g.BitBlt(mem,0,0,w,hgt,dc,r.left,r.top,0x00cc0020)
    g.SelectObject(mem,old);info=BI(C.sizeof(BI),w,-hgt,1,32,0,0,0,0,0,0);buf=C.create_string_buffer(w*hgt*4)
    assert g.GetDIBits(mem,bmp,0,hgt,buf,C.byref(info),0)
    im=Image.frombuffer('RGB',(w,hgt),buf,'raw','BGRX',0,1).copy();g.DeleteObject(bmp);g.DeleteDC(mem);u.ReleaseDC(None,dc);return im
try:
    deadline=time.monotonic()+20
    while time.monotonic()<deadline:
        handles=windows(p.pid)
        if 'Live TV! — Control Panel' in handles:break
        assert p.poll() is None;time.sleep(.2)
    video=next(h for t,h in handles.items() if 'Vulkan preview' in t);deck=handles['Live TV! — Control Panel']
    u.ShowWindow(deck,0);u.SetWindowPos(video,-1,200,160,1120,770,0x10);time.sleep(.35)
    baseline=screen_capture(video);baseline.save(root/'work'/f'move-{tag}-baseline.png')
    timer=u.GetDlgItem(video,118);u.KillTimer(video,1)
    u.SetWindowTextW(timer,'00:01:07');u.UpdateWindow(timer);time.sleep(.05)
    time_before=screen_capture(timer);time_before.save(root/'work'/f'timer-{tag}-before.png')
    u.SendMessageW(video,0x113,1,0);time.sleep(.1)
    time_after=screen_capture(timer);time_after.save(root/'work'/f'timer-{tag}-after.png')
    timer_corner_before=time_before.getpixel((4,4));timer_corner_after=time_after.getpixel((4,4))

    policy=C.c_int();enabled=W.BOOL();dw.DwmGetWindowAttribute(video,2,C.byref(policy),4);dw.DwmGetWindowAttribute(video,1,C.byref(enabled),4)
    # Exercise the native move loop using its keyboard form, then force the
    # activation/nonclient transitions that occur during ordinary window moves.
    first=rect(video)
    u.PostMessageW(video,0x112,0xf010,0);time.sleep(.1)
    for key in [0x27,0x27,0x28,0x0d]:u.PostMessageW(video,0x100,key,0);time.sleep(.07)
    second=rect(video)
    for i in range(6):
        u.SendMessageW(video,0x86,i%2,0)
        u.SendMessageW(video,0x85,1,0)
        u.SetWindowPos(video,None,210+i*7,180+i*5,0,0,0x15);time.sleep(.05)
    u.PostMessageW(video,0x232,0,0);time.sleep(.35)
    after=screen_capture(video);after.save(root/'work'/f'move-{tag}-after.png')
    # Sample only pixels inside the window's own rounded region at its outer rim.
    region=g.CreateRectRgn(0,0,0,0);u.GetWindowRgn(video,region)
    def gray_count(im):
        count=0
        for y in range(im.height):
            for x in (range(im.width) if y<8 or y>=im.height-8 else [*range(8),*range(im.width-8,im.width)]):
                if g.PtInRegion(region,x,y):
                    rgb=im.getpixel((x,y));count+=min(rgb)>160 and max(rgb)-min(rgb)<12
        return count
    result={'tag':tag,'native_move_changed_position':(first.left,first.top)!=(second.left,second.top),'nonclient_policy':policy.value,'nonclient_enabled':bool(enabled.value),'gray_rim_before':gray_count(baseline),'gray_rim_after':gray_count(after),'timer_background_before':timer_corner_before,'timer_background_after':timer_corner_after}
    g.DeleteObject(region)
    assert result['native_move_changed_position']
    assert result['gray_rim_after']==0,result
    assert timer_corner_before==timer_corner_after,result
    sheet=Image.new('RGB',(1120,100));sheet.paste(baseline.crop((0,0,1120,50)),(0,0));sheet.paste(after.crop((0,0,1120,50)),(0,50));sheet.save(root/'work'/f'move-{tag}-rim.png')
    (root/'work'/f'move-{tag}.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))
finally:
    u.PostMessageW(video,0x1f,0,0)
    p.terminate()
    try:p.wait(timeout=8)
    except subprocess.TimeoutExpired:p.kill();p.wait()
    log.close()
