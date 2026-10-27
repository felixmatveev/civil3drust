#!/usr/bin/env python3
"""Rebuild drawing B's record from drawing A's record and a hexview --diff
report for one record (--filter <handle>).

Usage: rebuild_from_report.py report.txt a_record.bin out.bin
(a_record.bin from `examples/dumprec`). Bits a report cut short are
filled with zeros and listed."""
import re,sys
t=open(sys.argv[1]).read(); prev=open(sys.argv[2],'rb').read(); out_path=sys.argv[3]
a=''.join(f'{x:08b}' for x in prev)
m=re.search(r'data stream: A (\d+) B (\d+) bits',t); alen,blen=int(m[1]),int(m[2])
regs=re.findall(r'\[\d+\] A bits (\d+)\.\.(\d+)\s+B bits (\d+)\.\.(\d+).*?\n\s+A: ?([01]*)[^\n]*\n\s+B: ?([01]*)([^\n]*)',t)
out='';p=0;filled=[]
for a0,a1,b0,b1,ab,bb,tail in regs:
    a0,a1,b0,b1=map(int,(a0,a1,b0,b1))
    assert len(out)+(a0-p)==b0
    if len(bb)<b1-b0: filled.append((b0+len(bb),b1)); bb=bb+'0'*(b1-b0-len(bb))
    out+=a[p:a0]+bb; p=a1
out+=a[p:alen]; assert len(out)==blen,(len(out),blen)
hs=re.search(r'handle stream \(A.*\n\s+A: [01]+\n\s+B: ([01]+)',t).group(1)
full=out+hs; assert len(full)%8==0
open(out_path,'wb').write(bytes(int(full[i:i+8],2) for i in range(0,len(full),8)))
print(len(full)//8,'bytes, handle bits',len(hs),'unknown (zero-filled) B ranges:',filled)
