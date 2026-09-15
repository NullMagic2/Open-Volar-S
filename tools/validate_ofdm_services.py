#!/usr/bin/env python3
"""Compare source-built services with a user-supplied reference; no USB writes.

Example: python validate_ofdm_services.py reference.fw open.fw --output result.json
The ROM oracle proves boundary equivalence under its declared models, not RF lock.
"""
import argparse,hashlib,json,random,itertools
from pathlib import Path
from it9175_boot_contract import parse_scatter,memory_map,decode
from mcs51_service_cpu import Cpu
REFERENCE='4b066157d0eb1a088e55daaf339418fc6596b5f76c4583d06d5eddb3a272e921'
SERVICES=[0x4DDA,0x503F,0x5153,0x554E,0x559B,0x55B6,0x578D,0x4EF6,0x4F13,0x4A1B,0x4DCF,0x4954,0x4AF1,0x4B1B]
END={0x4DDA:0x4EF0,0x503F:0x5153,0x5153:0x554E,0x554E:0x559B,0x559B:0x55B6,0x55B6:0x578D,0x578D:0x57BB,0x4EF6:0x4F13,0x4F13:0x503F,0x4A1B:0x4AF1,0x4DCF:0x4DDA,0x4954:0x4A1B,0x4AF1:0x4B1B,0x4B1B:0x4DCF}

def main():
 p=argparse.ArgumentParser();p.add_argument('reference',type=Path);p.add_argument('candidate',type=Path);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
 reference=args.reference.read_bytes();candidate=args.candidate.read_bytes()
 if hashlib.sha256(reference).hexdigest()!=REFERENCE:raise ValueError('Reference hash does not match investigated A865R firmware')
 rm,rp=memory_map(parse_scatter(reference)[1],1);om,op=memory_map(parse_scatter(candidate)[1],1)
 targets={}
 for v in range(0x6680,0x6800,3):
  r,o=decode(rm,v),decode(om,v)
  if r.target in SERVICES:targets[r.target]=o.target
 # Read explicit startup callback-map stores so service relocation is tested.
 callback={}
 for pc in range(0x4870,0x495B-5):
  if om[pc]==0x90 and om[pc+3]==0x74 and om[pc+5]==0xF0:
   callback[om[pc+1]*256+om[pc+2]]=om[pc+4]
 for slot,service in [(0,0x4B1B),(1,0x4AF1),(2,0x4A1B),(3,0x4954),(5,0x4DCF)]:
  address=0xFB5B+slot*2;targets[service]=callback[address]*256+callback[address+1]
 assert om[0x4700:0x47E6]==rm[0x4700:0x47E6], 'Startup data differs from reference'
 rng=random.Random(9175);base=bytearray(rng.randbytes(65536));cases=[]
 def add(service,changes={},r7=0,oracle=0,ready=0):cases.append((service,dict(changes),r7,oracle,ready))
 # Exhaust event selector space plus all important branch gates.
 for event in range(256):add(0x4DDA,{0x45F6:1,0x45BA:0,0x44E2:17,0x446D:1,0x45FD:0,0xF985:1,0xF9D9:3,0x4466:1,0xF5B0:1,0x42DE:1,0x4570:0,0x4571:0},event)
 for event in [0x1A,0x1B,0x1D,0x21,0x22,0x65]:
  for k in range(80):add(0x4DDA,{a:rng.choice([0,1,2,3,17,255]) for a in [0x45F6,0x45BA,0x44E2,0x446D,0x45FD,0xF985,0xF9D9,0x4466,0xF5B0,0x42DE,0x4570,0x4571]},event,k%3)
 for flags in itertools.product([0,1,2],repeat=4):
  for profile in [0,1,2]:
   for oracle in range(3):add(0x503F,dict(zip([0x4465,0x4466,0x44E3,0x44E9],flags))|{0x44E2:17 if oracle else 0,0x45F6:profile},oracle=oracle)
 for value in range(256):
  for offset in [0,1,4,5,128,255]:add(0x4EF6,{0x45EC:value,0x4588:offset})
 for selector in range(256):
  for override in [0,1]:add(0x4F13,{0x45FC:override,0xF903:1,0xF902:1},selector)
 for selector in [0,1,2,3]:
  for gates in itertools.product([0,1,2],repeat=3):add(0x4F13,dict(zip([0x45FC,0xF903,0xF902],gates)),selector)
 for oracle in range(3):add(0x554E,oracle=oracle)
 for i in range(2):add(0x559B,oracle=i);add(0x578D,{0x4322:i});add(0x4DCF,oracle=i)
 for unlocked in [0,1,2]:
  for armed in [0,1,2]:
   for offset in [0,1,0x3FFF,0x4000,0xFFFFFFFF,0xFFFFC001,0x80000000]:
    for oracle in [0,1]:
     changes={0x4466:unlocked,0x42AD:armed,0x45F0:255,0x45F1:63,0x45F4:1,0x45F5:0}
     changes.update({0x4232+i:v for i,v in enumerate(offset.to_bytes(4,'big'))});add(0x55B6,changes,oracle=oracle)
 for tick in [0,8,9,48,49,98,99,0xFFFE,0xFFFF]:
  for mode in [0,1,2,3]:
   for gate in [0,1]:
    for oracle in [0,1,2]:
     add(0x4A1B,{0x441D:tick>>8,0x441E:tick&255,0xF001:oracle,0x4320:gate,0x4609:mode,0x460F:gate,0xF9F1:gate,0xF9F2:oracle,0xF9F3:mode},oracle=oracle)
 for measurement,threshold in [(0,0),(0,1),(1,0),(256,257),(256,256),(65535,65535)]:
  for locked,stop in itertools.product([0,1],[0,1]):
   for ids in [(0,1,2,3),(4,5,4,5),(5,5,5,5),(4,0,5,2),(0,4,2,5),(5,4,5,4)]:
    changes={0x44B2:threshold&255,0x44B3:threshold>>8,0x45E9:threshold>>8,0x4466:locked,0x44CD:stop,0xF632:measurement&255,0xF633:measurement>>8,0x44B4:3}
    changes.update({0xF64C+i:v for i,v in enumerate(ids)});add(0x5153,changes,ready=2)
 # Threshold-to-tracking compare, no-tracking trigger, and greater-than range.
 for bound in [0,2,3,15,16,255]:
  for flag in [0,1]:add(0x5153,{0x44B2:1,0x44B3:0,0x4466:0,0xF632:0,0xF633:0,0x44B4:bound,0x44CD:0,0x42AF:flag,0xF64C:4,0xF64D:5,0xF64E:4,0xF64F:5})
 add(0x5153,{0x44B2:1,0x44B3:0,0x4466:0,0xF632:0,0xF633:0,0x44B4:255,0x44CD:0,0xF64C:4,0xF64D:5,0xF64E:4,0xF64F:5},oracle=3)
 for mode in [0,1,2]:
  for signal in range(256):
   for gate in [0,1]:
    add(0x4954,{0x4609:mode,0x44C3:signal,0xF9F1:gate,0xF906:signal,0xF90A:15 if gate else signal,0xF90E:signal if gate else 15},oracle=signal%4)
 for value in range(256):add(0x4AF1,{0xF903:value,0xF907:255-value,0xF90B:value^0x55})
 for bound,lanes,phase,frequency,flags,oracle in itertools.product([0,1,3,255,0xFFFF],[0,1],[0,1],[0,0x3FFFFF,0x400000,0xFFFFFF],[(0,0),(1,0),(0,1)],range(4)):
  changes={0x4236:bound>>8,0x4237:bound&255,0x421B:lanes,0x4477:phase,0xF1B5:frequency>>16,0xF1B4:(frequency>>8)&255,0xF1B3:frequency&255,0xF114:flags[0],0xF110:flags[1],0x4463:0,0xF5B7:1}
  add(0x4B1B,changes,oracle=oracle)
 for gate in [0,1]:
  add(0x4B1B,{0x4236:0,0x4237:1,0x421B:0,0x4477:1,0x4463:gate,0xF5B7:1},oracle=4+gate)
 counts={s:0 for s in SERVICES};covered={s:set() for s in SERVICES};failures=[]
 for index,(service,changes,r7,oracle,ready) in enumerate(cases):
  state=bytearray(base)
  for address,value in changes.items():state[address]=value
  left=Cpu(rm,rp,state,oracle,ready);right=Cpu(om,op,state,oracle,ready)
  if service==0x4B1B and oracle>=4:
   for cpu in [left,right]:
    cpu.read_scripts={0xF5B5:[0] if oracle==4 else [0,0,1],0xF5B7:[1] if oracle==4 else [0,0,1]}
  left.sr(7,r7);right.sr(7,r7)
  try:
   left.run(service);right.run(targets[service])
   if left.x!=right.x:
    diff=[f'{i:04x}:{left.x[i]:02x}/{right.x[i]:02x}' for i in range(65536) if left.x[i]!=right.x[i]]
    raise AssertionError('XDATA mismatch '+str(diff[:12]))
   if left.writes!=right.writes:raise AssertionError('MMIO write ordering mismatch '+str((left.writes[:20],right.writes[:20])))
   if left.trace!=right.trace:raise AssertionError('ROM boundary mismatch '+str((left.trace[:12],right.trace[:12])))
   for bit in [0xAF,0xCF,0x98,0x99]:
    if left.bit(bit)!=right.bit(bit):raise AssertionError(f'control bit {bit:02x} mismatch')
   counts[service]+=1;covered[service]|=left.visited
  except Exception as e:
   failures.append({'case':index,'entry':f'{service:04X}','changes':{f'{k:04X}':v for k,v in changes.items()},'r7':r7,'oracle':oracle,'error':str(e),'reference_pc':f'{left.pc:04X}','open_pc':f'{right.pc:04X}'})
   if len(failures)>=8:break
 timeout_ok=False
 search_timeouts=[]
 if not failures:
  state=bytearray(base);stuck=Cpu(om,op,state,ready_after=-1).run(targets[0x5153])
  timeout_ok=stuck.r(7)==255 and stuck.x[0xF625]==0 and stuck.d[0x81]==0x80
  for blocked in [0xF5B5,0xF5B7,0x423B]:
   state=bytearray(base)
   for address,value in {0x4236:0,0x4237:0,0x421B:1,0x4463:1,0xF5B7:1}.items():state[address]=value
   stuck=Cpu(om,op,state);stuck.read_scripts={blocked:[0]};stuck.run(targets[0x4B1B],limit=3000000)
   search_timeouts.append(stuck.r(7)==255 and stuck.x[0xF5B6]==0 and stuck.x[0xF5B0]==0 and stuck.d[0x81]==0x80)
  timeout_ok=timeout_ok and all(search_timeouts)
 report={'reference_sha256':REFERENCE,'candidate_sha256':hashlib.sha256(candidate).hexdigest(),'hardware_tested':False,'oracle_scope':'Declared ROM boundary models; no asynchronous interrupts, RF or electrical timing simulation. Scratch CPU registers/flags are excluded except EA, TF2, RI and TI.','cases_planned':len(cases),'cases_passed':sum(counts.values()),'bounded_poll_timeout_passed':timeout_ok,'services':[],'failures':failures}
 for s in SERVICES:
  addresses=[];pc=s
  while pc<END[s]:
   addresses.append(pc);instruction=decode(rm,pc);pc+=instruction.length
   if instruction.target==0x3AD3:pc+=4 # C51 inline literal, not instructions.
  report['services'].append({'reference':f'{s:04X}','candidate':f'{targets[s]:04X}','passed':counts[s],'reference_instructions':len(addresses),'covered_reference_instructions':len(set(addresses)&covered[s]),'uncovered':[f'{a:04X}' for a in addresses if a not in covered[s]]})
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({k:report[k] for k in ['cases_planned','cases_passed','bounded_poll_timeout_passed','failures']},indent=2));return 1 if failures or not timeout_ok else 0
if __name__=='__main__':raise SystemExit(main())

