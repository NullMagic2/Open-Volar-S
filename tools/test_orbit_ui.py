"""Exercise only the app process launched here, with an isolated settings folder."""
from pathlib import Path
import ctypes as C
from ctypes import wintypes as W
import subprocess,time,json,sys
from PIL import Image
source=Path(__file__).resolve().parents[1]
root=source/'target/orbit-ui-check'
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
log=open(root/'work/native-preview.log','w')
p=subprocess.Popen([str(source/'target/release/live-tv.exe'),'--ui-preview','--profile-dir',str(profile)],stdout=log,stderr=log,creationflags=0x08000000)
try:
    deadline=time.monotonic()+20
    while time.monotonic()<deadline:
        handles=windows(p.pid)
        if 'Live TV! — Control Panel' in handles and 'Live TV! — Settings' in handles:break
        assert p.poll() is None,(p.returncode,(root/'work/native-preview.log').read_text())
        time.sleep(.2)
    else:raise RuntimeError(handles)
    deck=handles['Live TV! — Control Panel'];settings=handles['Live TV! — Settings']
    video=next(h for t,h in handles.items() if 'Vulkan preview' in t)
    time.sleep(.7)
    u.GetWindowLongPtrW.restype=C.c_ssize_t;u.GetWindowLongPtrW.argtypes=[W.HWND,C.c_int]
    u.GetClientRect.argtypes=[W.HWND,C.POINTER(W.RECT)]
    border=W.RECT();client=W.RECT();u.GetWindowRect(deck,C.byref(border));u.GetClientRect(deck,C.byref(client))
    assert not (u.GetWindowLongPtrW(deck,-16)&0x00c00000),'System caption must be removed'
    assert client.right==border.right-border.left and client.bottom==border.bottom-border.top,'Native border must not consume client space'
    assert abs(client.right/client.bottom-1679/547)<.01,'Panel aspect must preserve the circular knob'
    def hit(x,y):return u.SendMessageW(deck,0x84,0,((y&65535)<<16)|(x&65535))
    assert hit(border.left+client.right//2,border.top+client.bottom*40//547)==2,'Header must drag'
    assert hit(border.left+1,border.top+client.bottom//2)==10,'Edge must resize'
    assert u.GetDlgItem(deck,113),"Close control must exist"
    close_rect=W.RECT();u.GetWindowRect(u.GetDlgItem(deck,113),C.byref(close_rect))
    assert hit((close_rect.left+close_rect.right)//2,(close_rect.top+close_rect.bottom)//2)==1,'Caption symbols must remain clickable'
    region=g.CreateRectRgn(0,0,0,0)
    assert u.GetWindowRgn(deck,region)>0,'Panel must clip the exterior corner fill'
    assert not g.PtInRegion(region,0,0),'Corner outside the rim must show the desktop'
    assert g.PtInRegion(region,client.right//2,client.bottom//2),'Panel interior must remain visible'
    g.DeleteObject(region)
    assert not u.GetDlgItem(deck,117),'Duplicate playback slider'
    assert u.GetDlgItem(video,117),'Video playback slider missing'
    back=W.RECT();forward=W.RECT();play=W.RECT()
    u.GetWindowRect(u.GetDlgItem(deck,120),C.byref(back));u.GetWindowRect(u.GetDlgItem(deck,121),C.byref(forward));u.GetWindowRect(u.GetDlgItem(deck,101),C.byref(play))
    assert back.right<forward.left and forward.right<play.left,'Seek controls must be adjacent before Play'
    def keys(hwnd,*values):
        for key in values:
            u.PostMessageW(hwnd,0x100,key,0);u.PostMessageW(hwnd,0x101,key,0);time.sleep(.05)
    def selected_channel():return json.loads((profile/'settings.json').read_text()).get('channel_index',0)
    keys(deck,0x30,0x37,0x0d);time.sleep(.2);assert selected_channel()==1,'07 Enter must select channel 7'
    keys(u.GetDlgItem(deck,120),0x30,0x34)
    commit_deadline=time.monotonic()+4
    while selected_channel()!=0 and time.monotonic()<commit_deadline:time.sleep(.05)
    assert selected_channel()==0,'04 timeout must select channel 4 from child focus'
    keys(deck,0x30,0x37,0x1b);time.sleep(1.6);assert selected_channel()==0,'Escape must cancel entry'
    keys(deck,0x39,0x39,0x0d);time.sleep(.2);assert selected_channel()==0,'Unknown channel must not retune'
    keys(deck,0x30,0x37,0x08,0x34,0x0d);time.sleep(.2);assert selected_channel()==0,'Backspace must edit the number'
    # A dropdown highlight is provisional; only committing closes and selects it.
    channel_combo=u.GetDlgItem(deck,116)
    u.SendMessageW(channel_combo,0x14f,1,0);keys(channel_combo,0x28)
    assert selected_channel()==0,'Highlighting a dropdown row must not tune or rebuild its list'
    keys(channel_combo,0x0d)
    deadline=time.monotonic()+3
    while selected_channel()!=1 and time.monotonic()<deadline:time.sleep(.05)
    assert selected_channel()==1,'Enter must commit the chosen dropdown channel'
    for _ in range(8):
        assert not u.SendMessageW(channel_combo,0x157,0,0),'Channel dropdown must stay closed after selection'
        time.sleep(.15)
    u.SendMessageW(channel_combo,0x14f,1,0);keys(channel_combo,0x26,0x1b);time.sleep(.2)
    assert selected_channel()==1 and not u.SendMessageW(channel_combo,0x157,0,0),'Escape must cancel dropdown selection'
    def saved_volume():return json.loads((profile/'settings.json').read_text())['volume']
    original_volume=saved_volume()
    keys(video,0x26);time.sleep(.2);assert selected_channel()==0 and saved_volume()==original_volume,'Stream Up must select the next channel'
    keys(u.GetDlgItem(video,101),0x28);time.sleep(.2);assert selected_channel()==1,'Stream Down must select previous channel from child focus'
    keys(u.GetDlgItem(video,117),0x25);time.sleep(.2);assert saved_volume()==max(0,original_volume-5) and selected_channel()==1,'Stream Left must lower volume, including from slider focus'
    keys(video,0x27,0x26);time.sleep(.2);assert saved_volume()==original_volume and selected_channel()==0,'Stream Right must raise volume; Up must restore channel 4'
    osd=u.GetDlgItem(video,501);osd_text=C.create_unicode_buffer(512)
    u.SendMessageW(osd,0x0d,512,C.addressof(osd_text))
    assert osd_text.value=='4 – TV GAZETA HD\nSignal quality: —',osd_text.value
    capture(video,'Live-TV-Channel-Overlay.png')
    u.SendMessageW(deck,0x111,109,0)
    settings_rect=W.RECT();u.GetWindowRect(settings,C.byref(settings_rect))
    assert abs((settings_rect.right-settings_rect.left)/(settings_rect.bottom-settings_rect.top)-610/580)<.01,'Options window proportions must not revert to the old tall layout'
    assert not (u.GetWindowLongPtrW(settings,-16)&0x00c00000),'Settings must be borderless'
    assert not u.GetWindowLongPtrW(settings,-8),'Settings must have no owner that gets raised with it'
    class MonitorInfo(C.Structure):_fields_=[('size',W.DWORD),('monitor',W.RECT),('work',W.RECT),('flags',W.DWORD)]
    u.MonitorFromWindow.restype=W.HANDLE;u.MonitorFromWindow.argtypes=[W.HWND,W.DWORD]
    u.GetMonitorInfoW.argtypes=[W.HANDLE,C.POINTER(MonitorInfo)]
    mi=MonitorInfo();mi.size=C.sizeof(mi);assert u.GetMonitorInfoW(u.MonitorFromWindow(deck,2),C.byref(mi))
    panel_rect=W.RECT();u.GetWindowRect(deck,C.byref(panel_rect))
    assert abs((panel_rect.left+panel_rect.right)-(mi.work.left+mi.work.right))<=1 and abs((panel_rect.top+panel_rect.bottom)-(mi.work.top+mi.work.bottom))<=1,'Panel must start centered in the screen work area'
    sw=settings_rect.right-settings_rect.left;sh=settings_rect.bottom-settings_rect.top
    expected_x=max(mi.work.left,min((panel_rect.left+panel_rect.right-sw)//2,mi.work.right-sw))
    expected_y=max(mi.work.top,min((panel_rect.top+panel_rect.bottom-sh)//2,mi.work.bottom-sh))
    assert abs(settings_rect.left-expected_x)<=1 and abs(settings_rect.top-expected_y)<=1,'Settings must center on the panel within the work area'
    sr=W.RECT();u.GetClientRect(settings,C.byref(sr))
    assert sr.right==settings_rect.right-settings_rect.left and sr.bottom==settings_rect.bottom-settings_rect.top
    region=g.CreateRectRgn(0,0,0,0)
    assert u.GetWindowRgn(settings,region)>0 and not g.PtInRegion(region,0,0),'Settings corner must expose desktop'
    g.DeleteObject(region)
    assert u.GetDlgItem(settings,113),'Settings needs the screw close control'
    def settings_hit(x,y):
        return u.SendMessageW(settings,0x84,0,(((settings_rect.top+y)&65535)<<16)|((settings_rect.left+x)&65535))
    scale=u.GetDpiForWindow(settings)/96
    assert settings_hit(int(100*scale),int(24*scale))==2,'Settings header must drag'
    assert settings_hit(int(50*scale),int(530*scale))==2,'Exposed Settings fascia must drag'
    assert settings_hit(int(590*scale),int(250*scale))==1,'Page interior stays client area'
    assert settings_hit(sr.right-int(49*scale),int(24*scale))==1,'Close screw stays clickable'
    assert settings_hit(int(465*scale),sr.bottom-int(49*scale))==1,'Apply stays clickable'

    tabs=u.GetDlgItem(settings,320)
    for index in range(4):
        if index:
            u.SendMessageW(tabs,0x100,0x27,0)
            u.SendMessageW(tabs,0x101,0x27,0)
        time.sleep(.15)
        print('tab',index,'selected',u.SendMessageW(tabs,0x130b,0,0),'appearance visible',u.IsWindowVisible(u.GetDlgItem(settings,322)),flush=True)
        assert u.SendMessageW(tabs,0x130b,0,0)==index
        assert bool(u.IsWindowVisible(u.GetDlgItem(settings,322)))==(index==3)
        if index==0:
            check_dropdown_backdrop(settings,u.GetDlgItem(settings,301),u.GetDlgItem(settings,302))
            capture(settings,'Live-TV-Settings-Video.png')
        if index==2:
            def field_text(id):
                buf=C.create_unicode_buffer(4096);u.SendMessageW(u.GetDlgItem(settings,id),0x0d,4096,C.addressof(buf));return buf.value
            def set_field(id,value):
                buf=C.create_unicode_buffer(value);u.SendMessageW(u.GetDlgItem(settings,id),0x0c,0,C.addressof(buf))
            assert not (u.GetWindowLongPtrW(u.GetDlgItem(settings,314),-16)&0x800),'Recording path must be editable'
            assert not (u.GetWindowLongPtrW(u.GetDlgItem(settings,315),-16)&0x800),'Snapshot path must be editable'
            set_field(315,'relative');u.SendMessageW(settings,0x111,201,0)
            assert u.IsWindowVisible(settings) and 'full folder path' in field_text(316),'Invalid paths must keep Storage open with an explanation'
            recording_test_path=str(profile/'saved recordings');snapshot_test_path=str(profile/'saved snapshots')
            set_field(314,recording_test_path);set_field(315,snapshot_test_path)
            u.SendMessageW(settings,0x111,201,0)
            stored=json.loads((profile/'settings.json').read_text())
            assert stored['recording_folder']==recording_test_path and stored['snapshot_folder']==snapshot_test_path,'Apply must save both folders'
            u.SendMessageW(deck,0x111,109,0)
            assert field_text(314)==recording_test_path and field_text(315)==snapshot_test_path
            capture(settings,'Live-TV-Settings-Storage.png')

        if index==1:
            status=C.create_unicode_buffer(256);u.SendMessageW(u.GetDlgItem(settings,307),0x0d,256,C.addressof(status))
            assert 'Preparing playback' not in status.value,'Idle settings must not retain preparation text'
            country=u.GetDlgItem(settings,311)
            class ComboInfo(C.Structure):
                _fields_=[('size',W.DWORD),('item',W.RECT),('button',W.RECT),('state',W.DWORD),('combo',W.HWND),('edit',W.HWND),('list',W.HWND)]
            info=ComboInfo();info.size=C.sizeof(info)
            u.GetComboBoxInfo.argtypes=[W.HWND,C.POINTER(ComboInfo)]
            assert u.GetComboBoxInfo(country,C.byref(info)) and info.edit
            cr=W.RECT();er=W.RECT();u.GetWindowRect(country,C.byref(cr));u.GetWindowRect(info.edit,C.byref(er))
            assert er.left>cr.left+3 and er.top>cr.top+3 and er.bottom<cr.bottom-3,'Country text must stay inside the frame'
            original=C.create_unicode_buffer(256);u.SendMessageW(info.edit,0x0d,256,C.addressof(original))
            trial=C.create_unicode_buffer('Test country');u.SendMessageW(info.edit,0x0c,0,C.addressof(trial))
            value=C.create_unicode_buffer(256);u.SendMessageW(country,0x0d,256,C.addressof(value));assert value.value=='Test country'
            u.SendMessageW(info.edit,0x0c,0,C.addressof(original))
            u.SendMessageW(country,0x14f,1,0);assert u.SendMessageW(country,0x157,0,0)==1
            u.SendMessageW(country,0x14f,0,0)
            time.sleep(.1)
            u.GetWindowRect(info.edit,C.byref(er))
            assert er.top>cr.top+3 and er.bottom<cr.bottom-3,'Dropdown close must preserve the text inset'
            capture(settings,'Live-TV-Settings-Channels.png')
    u.SendMessageW(u.GetDlgItem(settings,113),0xf5,0,0);assert not u.IsWindowVisible(settings)
    u.SendMessageW(deck,0x111,109,0);assert u.IsWindowVisible(settings)
    material=u.GetDlgItem(settings,322)
    for index,name in enumerate(['metal','glass','plastic']):
        u.SendMessageW(material,0x14e,index,0)
        u.SendMessageW(settings,0x111,322|(1<<16),material);time.sleep(.15)
        saved=json.loads((profile/'settings.json').read_text());assert saved['theme']=='orbit' and saved['button_material']==name
        capture(deck,f'Live-TV-Orbit-{name}.png')
        if '--layout-only' not in sys.argv:
            u.ShowWindow(settings,5)
            if index==0:capture(settings,'Live-TV-Settings-Appearance.png')
            hover_tiles=[]
            cc=u.GetDlgItem(deck,124)
            u.SendMessageW(cc,0xf5,0,0) # Exercise hover with captions initially off, then verify the active state.
            for control,label in [(103,'REC'),(124,'CC'),(122,'LIVE')]:
                button=u.GetDlgItem(deck,control)
                was_enabled=bool(u.IsWindowEnabled(button));u.EnableWindow(button,True)
                u.ShowWindow(settings,0)
                u.SetWindowPos(deck,W.HWND(-1),0,0,0,0,0x13)
                br=W.RECT();u.GetWindowRect(button,C.byref(br))
                test_cursor=(border.left+client.right//2,border.top+20)
                u.SetCursorPos(*test_cursor)
                u.SendMessageW(button,0x2a3,0,0);time.sleep(.3)
                before=capture(button)
                def accent_count(im,label,labels_only=False):
                    area=im.crop((im.width//2 if labels_only else 12,8,im.width-12,im.height-8))
                    # Check pigment cores, excluding ClearType's colored fringes on neutral text.
                    pigment=((217,106,85) if name=='glass' else (152,47,37)) if label in ('REC','LIVE') else ((197,163,55) if name=='glass' else (145,109,6))
                    return sum(max(abs(v-t) for v,t in zip(pixel,pigment))<10 for pixel in area.getdata())
                if accent_count(before,label,True)>=12:before.save(root/'outputs/Live-TV-Hover-Details.png')
                assert accent_count(before,label,True)<12,f'{name} {label} label must be neutral at rest'
                if label in ('REC','LIVE'):
                    assert accent_count(before,label)>20,f'{name} {label} icon must stay illuminated at rest'
                test_cursor=((br.left+br.right)//2,(br.top+br.bottom)//2)
                u.SetCursorPos(*test_cursor)
                for _ in range(18):
                    u.EnableWindow(button,True)
                    u.SendMessageW(button,0x200,0,(8<<16)|8);time.sleep(.016)
                # Synthetic pointer messages must not leave native tracking tied to
                # the user's real cursor. Cancel tracking and sample the actual label.
                class TrackMouse(C.Structure):_fields_=[('size',W.DWORD),('flags',W.DWORD),('window',W.HWND),('hover_time',W.DWORD)]
                track=TrackMouse(C.sizeof(TrackMouse),0x80000002,button,0)
                for attempt in range(10):
                    u.EnableWindow(button,True)
                    u.SendMessageW(button,0x200,0,(8<<16)|8)
                    u.TrackMouseEvent(C.byref(track))
                    u.UpdateWindow(button)
                    hovered=capture(button)
                    if accent_count(hovered,label,True)>4:break
                hovered.save(root/'outputs/Live-TV-Hover-Details.png')
                assert accent_count(hovered,label,True)>4,f'{name} {label} text must take its accent on hover'
                hover_tiles.extend([before,hovered])
                u.SetCursorPos(border.left+client.right//2,border.top+20)
                test_cursor=(border.left+client.right//2,border.top+20)
                u.SendMessageW(button,0x2a3,0,0);time.sleep(.25)
                u.EnableWindow(button,was_enabled)
            strip=Image.new('RGB',(max(i.width for i in hover_tiles)*2,max(i.height for i in hover_tiles)*3),'#251b13')
            for n,tile in enumerate(hover_tiles):strip.paste(tile,((n%2)*strip.width//2,(n//2)*strip.height//3))
            strip.save(root/'outputs/Live-TV-Hover-Details.png')
            u.SendMessageW(cc,0xf5,0,0)
            on=capture(cc)
            pixels=list(on.crop((12,8,on.width-12,on.height-8)).getdata())
            assert accent_count(on,'CC')>20,'Enabled captions must retain the yellow icon'
            assert accent_count(on,'CC',True)<12,'Enabled captions must have a neutral label without hover'

    u.SetWindowPos(deck,W.HWND(-2),0,0,0,0,0x13)
    # Guide uses fixture data only; opening it never tunes or starts playback.
    u.SendMessageW(deck,0x111,119,0);time.sleep(.2)
    guide=windows(p.pid).get('Live TV! — Program guide');assert guide,'Program guide must open'
    assert not (u.GetWindowLongPtrW(guide,-16)&0x00c00000),'Guide must not have a native caption'
    gr=W.RECT();gc=W.RECT();u.GetWindowRect(guide,C.byref(gr));u.GetClientRect(guide,C.byref(gc))
    assert gc.right==gr.right-gr.left and gc.bottom==gr.bottom-gr.top,'Guide must have no native frame inset'
    guide_list=u.GetDlgItem(guide,602);guide_filter=u.GetDlgItem(guide,601)
    check_dropdown_backdrop(guide,guide_filter,guide_list)
    assert u.SendMessageW(guide_list,0x18b,0,0)==24,'Guide must load all fixture events'
    u.SendMessageW(guide_filter,0x14e,1,0);u.SendMessageW(guide,0x111,601|(1<<16),guide_filter)
    assert u.SendMessageW(guide_list,0x18b,0,0)==12,'Guide filtering must retain the station events'
    u.SendMessageW(guide_list,0x186,1,0);u.SendMessageW(guide,0x111,602|(1<<16),guide_list)
    detail=C.create_unicode_buffer(4096);u.SendMessageW(u.GetDlgItem(guide,603),0x0d,4096,C.addressof(detail))
    assert 'Program information supplied' in detail.value,'Selection must populate the description'
    capture(guide,'Live-TV-Program-Guide.png')
    region=g.CreateRectRgn(0,0,0,0)
    assert u.GetWindowRgn(guide,region)>0 and not g.PtInRegion(region,0,0),'Guide corners must expose desktop'
    g.DeleteObject(region)
    u.SendMessageW(u.GetDlgItem(guide,113),0xf5,0,0);assert not u.IsWindowVisible(guide),'Guide close screw must hide it'

    check_dropdown_backdrop(deck,u.GetDlgItem(deck,116),u.GetDlgItem(deck,109))
    # The native selector still opens a list; no search UI is introduced.
    combo=u.GetDlgItem(deck,116);u.SendMessageW(combo,0x14f,1,0);time.sleep(.1)
    assert u.SendMessageW(combo,0x157,0,0)==1
    u.SendMessageW(combo,0x14f,0,0)
    u.SendMessageW(u.GetDlgItem(deck,114),0xf5,0,0);time.sleep(.1)
    assert u.IsIconic(deck),'Etched minimize control'
    u.ShowWindow(deck,9);time.sleep(.1)
    u.SendMessageW(u.GetDlgItem(deck,113),0xf5,0,0);time.sleep(.1)
    assert not u.IsWindowVisible(deck) and u.IsWindow(video),'Close hides only the panel'
    u.SendMessageW(video,0x111,112,0);time.sleep(.1)
    assert u.IsWindowVisible(deck),'Panel can be reopened'
    # Verify the saved material is loaded on a fresh application start.
    u.PostMessageW(video,0x10,0,0);p.wait(timeout=5)
    p=subprocess.Popen([str(source/'target/release/live-tv.exe'),'--ui-preview','--profile-dir',str(profile)],stdout=log,stderr=log,creationflags=0x08000000)
    deadline=time.monotonic()+10
    while time.monotonic()<deadline:
        handles=windows(p.pid)
        if 'Live TV! — Control Panel' in handles and 'Live TV! — Settings' in handles:break
        time.sleep(.1)
    deck=handles['Live TV! — Control Panel'];settings=handles['Live TV! — Settings'];time.sleep(.3)
    u.SendMessageW(deck,0x111,109,0)
    assert u.SendMessageW(u.GetDlgItem(settings,322),0x147,0,0)==2
    for field,expected in [(314,recording_test_path),(315,snapshot_test_path)]:
        buf=C.create_unicode_buffer(4096);u.SendMessageW(u.GetDlgItem(settings,field),0x0d,4096,C.addressof(buf));assert buf.value==expected,'Storage folders must survive restart'

    print('Native layout checks passed: centered startup panel, Settings centering/ownership, tabs, guide, channel entry, and control layout.' if '--layout-only' in sys.argv else 'Native smoke checks passed: hover accents, illuminated icons, centered windows, tabs, guide, channel entry, and control layout.')
finally:
    cursor=W.POINT();u.GetCursorPos(C.byref(cursor))
    if test_cursor==(cursor.x,cursor.y):u.SetCursorPos(original_cursor.x,original_cursor.y)
    for title,h in windows(p.pid).items():
        if 'Vulkan preview' in title:u.PostMessageW(h,0x10,0,0)
    try:p.wait(timeout=5)
    except subprocess.TimeoutExpired:p.terminate();p.wait(timeout=5)
    log.close()
