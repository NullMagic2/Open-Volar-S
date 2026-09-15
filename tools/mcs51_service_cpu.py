"""Bounded MCS-51 differential executor for downloaded IT9175 services.

This is not a device simulator. Unknown ROM bodies are external deterministic
oracles; RF/MMIO timing, interrupts and electrical behavior are not modeled.
Instructions fail closed when unsupported. No reference payload is bundled.
"""
from dataclasses import dataclass

class Cpu:
 def __init__(self,code,present,xdata,oracle=0,ready_after=0):
  self.code,self.present=code,present;self.x=bytearray(xdata);self.d=bytearray(256);self.stack_ram=bytearray(256)
  self.d[0x81]=0x80;self.d[0x98]=0x50;self.d[0xA8]=0x80
  self.pc=0;self.depth=0;self.done=False;self.steps=0;self.trace=[];self.writes=[]
  self.oracle=oracle;self.ready_after=ready_after;self.polls=0;self.visited=set();self.read_scripts={}
 @property
 def a(self):return self.d[0xE0]
 @a.setter
 def a(self,v):self.d[0xE0]=v&255
 @property
 def c(self):return self.d[0xD0]>>7
 @c.setter
 def c(self,v):self.d[0xD0]=(self.d[0xD0]&127)|(bool(v)<<7)
 @property
 def dp(self):return self.d[0x82]|self.d[0x83]<<8
 @dp.setter
 def dp(self,v):self.d[0x82]=v&255;self.d[0x83]=(v>>8)&255
 def ri(self,r):return (self.d[0xD0]&0x18)+r
 def r(self,r):return self.d[self.ri(r)]
 def sr(self,r,v):self.d[self.ri(r)]=v&255
 def bit(self,b):return (self.d[(b&0xF8) if b>=128 else 0x20+(b>>3)]>>(b&7))&1
 def sb(self,b,v):
  i=(b&0xF8) if b>=128 else 0x20+(b>>3);self.d[i]=(self.d[i]&~(1<<(b&7)))|(bool(v)<<(b&7))
 def push(self,v):self.d[0x81]=(self.d[0x81]+1)&255;self.stack_ram[self.d[0x81]]=v&255
 def pop(self):v=self.stack_ram[self.d[0x81]];self.d[0x81]=(self.d[0x81]-1)&255;return v
 def read(self,a):
  if a in self.read_scripts:
   script=self.read_scripts[a];value=script.pop(0) if len(script)>1 else script[0]
   self.x[a]=value
   return value
  if a==0xF62F:
   self.polls+=1
   if self.ready_after<0 or self.polls<=self.ready_after:return 0
  return self.x[a]
 def write(self,a,v):
  self.x[a]=v&255
  if a>=0xE000:self.writes.append((a,v&255))
 def long(self,r):return int.from_bytes(bytes(self.r(i) for i in range(r,r+4)),'big')
 def ret(self):
  if self.depth==0:self.done=True
  else:self.pc=(self.pop()<<8)|self.pop();self.depth-=1
 def rom(self,target):
  # C51 utility oracles are tested with signed and unsigned comparison modes.
  # Keeping identical arguments means correctness does not depend on choosing
  # an undocumented comparison interpretation for the replacement.
  if target==0x3B04:
   for i in range(4):self.write((self.dp+i)&65535,self.r(4+i))
   self.dp=(self.dp+3)&65535
  elif target==0x3B46:
   left,right=self.long(0),self.long(4)
   borrow=self.c;self.trace.append((target,left,right,borrow))
   if self.oracle%2:left=left-(1<<32) if left>>31 else left;right=right-(1<<32) if right>>31 else right
   self.c=left<right+borrow
  elif target==0x3AD3:
   hi=self.pop();lo=self.pop();address=(hi<<8)|lo
   for i in range(4):self.write(self.dp+i,self.code[address+i])
   self.dp+=3;address+=4;self.push(address&255);self.push(address>>8)
  elif target==0x3AB2:
   value=(-self.long(4))&0xFFFFFFFF
   for i,v in enumerate(value.to_bytes(4,'big')):self.sr(4+i,v)
  elif target==0x303A:self.trace.append((target,self.r(7),self.r(5),self.x[0x45FC]))
  elif target==0xDC8D:
   self.trace.append((target,self.r(7)));self.x[0x43EC]=(self.r(7)*37+self.oracle*43)&255
  elif target==0x721F:
   self.trace.append((target,self.r(6),self.r(7),self.r(5)))
   self.x[0x4228:0x4230]=(7).to_bytes(4,'big')+(self.oracle%3+6).to_bytes(4,'big')
   if self.oracle==3:self.x[0x45FF]=1
  elif target==0xAA9D:self.trace.append((target,))
  elif target==0x3D2C:self.trace.append((target,self.r(1),self.r(2),self.r(3),self.r(4),self.r(5),self.d[0x23]))
  elif target==0x3B70:
   value=self.r(6)*256+self.r(7);divisor=self.r(4)*256+self.r(5)
   self.trace.append((target,value,divisor));q,m=divmod(value,divisor)
   self.sr(6,q>>8);self.sr(7,q);self.sr(4,m>>8);self.sr(5,m)
  elif target==0xE3CC:
   self.trace.append((target,self.r(6),self.r(7),self.d[0x99]))
   if self.oracle%3==1:self.d[0x99]^=0xFF
  elif target in (0x97F3,0xB10B):self.trace.append((target,self.r(7)))
  elif target==0x9148:self.trace.append((target,self.r(7),bytes(self.x[0x42B0:0x42B4]).hex()))
  elif target in {0xD210,0xD7AC,0xD0E1,0xCC33,0xCDC3,0xCD32,0xD68F,0xD615,0xCCBC,0x75B8,0xB66C,0x9698,0xDF34,0xDB03,0xB4EF}:
   self.trace.append((target,))
   if target==0xB4EF and self.oracle==3:self.x[0x42AF]=1
   # Deterministic post-call state changes exercise reads after opaque ROM calls.
   if self.oracle%3==2 and target in {0xCC33,0xCDC3,0xCD32,0xD68F,0xD615,0xCCBC}:
    self.x[0x45F6]=1;self.x[0x45F7:0x45FB]=bytes([17,23,29,31])
  else:raise AssertionError(f'Unmodeled ROM boundary {target:04x}')
  self.ret()
 def run(self,entry,limit=1000000):
  self.pc=entry
  while not self.done:
   if self.steps>=limit:raise TimeoutError(f'step limit at {self.pc:04x}')
   self.steps+=1;self.visited.add(self.pc)
   if not self.present[self.pc]:self.rom(self.pc);continue
   p=self.pc;o=self.code[p];b=self.code[(p+1)&65535];c=self.code[(p+2)&65535];self.pc=(p+1)&65535
   rel=lambda n,v:(p+n+(v if v<128 else v-256))&65535
   if o==0:pass
   elif o&0x1F in (1,0x11):
    target=((p+2)&0xF800)|((o&0xE0)<<3)|b
    if o&0x1F==0x11:self.push((p+2)&255);self.push((p+2)>>8);self.depth+=1
    self.pc=target
   elif o in (2,0x12):
    if o==0x12:self.push((p+3)&255);self.push((p+3)>>8);self.depth+=1
    self.pc=b*256+c
   elif o in (0x22,0x32):self.ret()
   elif o==0x80:self.pc=rel(2,b)
   elif o==0x90:self.dp=b*256+c;self.pc=p+3
   elif o==0xA3:self.dp=(self.dp+1)&65535
   elif o==0x93:
    address=(self.dp+self.a)&65535
    if not self.present[address]:raise AssertionError(f'MOVC reads absent code {address:04x}')
    self.a=self.code[address]
   elif o==0xA4:
    value=self.a*self.d[0xF0];self.a=value;self.d[0xF0]=value>>8;self.c=0;self.sb(0xD2,value>255)
   elif o==0x33:
    value=self.a;carry=self.c;self.a=(value<<1)|carry;self.c=value>>7
   elif o==0xE0:self.a=self.read(self.dp)
   elif o==0xF0:self.write(self.dp,self.a)
   elif o==0x74:self.a=b;self.pc=p+2
   elif o==0x75:self.d[b]=c;self.pc=p+3
   elif o==0x85:self.d[c]=self.d[b];self.pc=p+3
   elif o==0xE5:self.a=self.d[b];self.pc=p+2
   elif o==0xF5:self.d[b]=self.a;self.pc=p+2
   elif 0xE8<=o<=0xEF:self.a=self.r(o&7)
   elif 0xF8<=o<=0xFF:self.sr(o&7,self.a)
   elif 0x78<=o<=0x7F:self.sr(o&7,b);self.pc=p+2
   elif 0x88<=o<=0x8F:self.d[b]=self.r(o&7);self.pc=p+2
   elif 0xA8<=o<=0xAF:self.sr(o&7,self.d[b]);self.pc=p+2
   elif o in (0xE4,0xF4):self.a=0 if o==0xE4 else ~self.a
   elif o in (4,0x14):self.a=self.a+(1 if o==4 else -1)
   elif o in (5,0x15):self.d[b]=(self.d[b]+(1 if o==5 else -1))&255;self.pc=p+2
   elif 8<=o<=15 or 0x18<=o<=0x1F:self.sr(o&7,self.r(o&7)+(1 if o<16 else -1))
   elif o in (0xC3,0xD3):self.c=o==0xD3
   elif o in (0xC2,0xD2):self.sb(b,o==0xD2);self.pc=p+2
   elif o in (0xC0,0xD0):
    if o==0xC0:self.push(self.d[b])
    else:self.d[b]=self.pop()
    self.pc=p+2
   elif o in (0x40,0x50,0x60,0x70):
    cond={0x40:bool(self.c),0x50:not self.c,0x60:self.a==0,0x70:self.a!=0}[o];self.pc=rel(2,b) if cond else p+2
   elif o in (0x20,0x30):self.pc=rel(3,c) if bool(self.bit(b))==(o==0x20) else p+3
   elif 0xB4<=o<=0xBF:
    if o==0xB4:v,w=self.a,b
    elif o==0xB5:v,w=self.a,self.d[b]
    elif o<0xB8:v,w=self.d[self.r(o&1)],b
    else:v,w=self.r(o&7),b
    self.c=v<w;self.pc=rel(3,c) if v!=w else p+3
   elif 0xD8<=o<=0xDF:self.sr(o&7,self.r(o&7)-1);self.pc=rel(2,b) if self.r(o&7) else p+2
   elif o==0xD5:self.d[b]=(self.d[b]-1)&255;self.pc=rel(3,c) if self.d[b] else p+3
   elif any(lo<=o<=hi for lo,hi in [(0x24,0x2F),(0x34,0x3F),(0x44,0x4F),(0x54,0x5F),(0x64,0x6F),(0x94,0x9F)]):
    mode=o&15;v=b if mode==4 else self.d[b] if mode==5 else self.d[self.r(mode&1)] if mode in (6,7) else self.r(mode&7)
    self.pc=p+(2 if mode<6 else 1);kind=o>>4;old=self.a
    if kind in (2,3):v=old+v+(self.c if kind==3 else 0);self.c=v>255;self.a=v
    elif kind==9:v=old-v-self.c;self.c=v<0;self.a=v
    elif kind==4:self.a=old|v
    elif kind==5:self.a=old&v
    elif kind==6:self.a=old^v
   else:raise AssertionError(f'unsupported instruction {o:02x} at {p:04x}')
  return self

