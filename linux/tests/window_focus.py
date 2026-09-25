"""Run under xvfb-run: verify real X11 stacking and keyboard focus for both windows."""
import ctypes as C
import re
import subprocess
import time
from pathlib import Path

root = Path(__file__).resolve().parents[2]
x = C.CDLL('libX11.so.6')
x.XOpenDisplay.argtypes=[C.c_char_p]; x.XOpenDisplay.restype=C.c_void_p
display=x.XOpenDisplay(None)
if not display: raise RuntimeError('X display required')
x.XDefaultRootWindow.argtypes=[C.c_void_p];x.XDefaultRootWindow.restype=C.c_ulong
screen=x.XDefaultRootWindow(display)
for name in ['XRaiseWindow','XUnmapWindow','XMapWindow','XDestroyWindow']:
 getattr(x,name).argtypes=[C.c_void_p,C.c_ulong]
x.XFlush.argtypes=[C.c_void_p]
x.XSetInputFocus.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_ulong]
x.XGetInputFocus.argtypes=[C.c_void_p,C.POINTER(C.c_ulong),C.POINTER(C.c_int)]
x.XQueryTree.argtypes=[C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ulong)),C.POINTER(C.c_uint)]
x.XFree.argtypes=[C.c_void_p]
x.XCreateSimpleWindow.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_int,C.c_uint,C.c_uint,C.c_uint,C.c_ulong,C.c_ulong]
x.XCreateSimpleWindow.restype=C.c_ulong

def order():
 r=C.c_ulong();p=C.c_ulong();children=C.POINTER(C.c_ulong)();count=C.c_uint()
 assert x.XQueryTree(display,screen,C.byref(r),C.byref(p),C.byref(children),C.byref(count))
 values=list(children[:count.value]);x.XFree(children);return values

def focus(window):
 x.XSetInputFocus(display,window,1,0);x.XFlush(display);time.sleep(.25)

process=subprocess.Popen([str(root/'target/release/open-volar-s-live-tv')],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
other=0
try:
 for _ in range(100):
  listing=subprocess.check_output(['xwininfo','-root','-tree'],text=True)
  windows=[]
  for line in listing.splitlines():
   m=re.search(r'(0x[0-9a-f]+) "Live TV![^"]*".*? (\d+)x(\d+)[+-]',line)
   if m and int(m[2])>100:windows.append((int(m[3]),int(m[1],16)))
  if len(windows)==2:break
  time.sleep(.1)
 assert len(windows)==2,listing
 windows.sort();deck=windows[0][1];viewer=windows[1][1]
 time.sleep(1.2)
 other=x.XCreateSimpleWindow(display,screen,20,20,200,200,0,0,0xffffff)
 x.XMapWindow(display,other);x.XFlush(display);time.sleep(.2)
 for active,peer in [(viewer,deck),(deck,viewer)]:
  x.XRaiseWindow(display,other);focus(other);focus(active)
  stacking=order()
  assert stacking.index(other)<stacking.index(peer)<stacking.index(active),stacking
  current=C.c_ulong();revert=C.c_int();x.XGetInputFocus(display,C.byref(current),C.byref(revert))
  assert current.value==active,(current.value,active)
  print('PASS: visible pair raised; keyboard focus remains on activated window',flush=True)
 x.XUnmapWindow(display,deck);x.XFlush(display);time.sleep(.2)
 focus(other);focus(viewer)
 info=subprocess.check_output(['xwininfo','-id',hex(deck)],text=True)
 assert 'IsUnMapped' in info,info
 print('PASS: unmapped DAC remains hidden',flush=True)
finally:
 if other:x.XDestroyWindow(display,other)
 process.terminate()
 try:process.wait(timeout=5)
 except subprocess.TimeoutExpired:process.kill();process.wait()
