#!/usr/bin/env python3
"""Synthetic real-stride arrays with sparse fields; not actual registry loading."""
RESOURCE_TABLE,SETTINGS_TABLE=0x146124398,0x1461243b0
RESOURCE_STRIDE,SETTINGS_STRIDE=0x6668,0x433c0
INDICES=(-1,0,1,5,20,77,78,88,89)
class Tables:
 def __init__(self,e,root_strings=False):
  self.root_strings=root_strings
  self.e=e;self.resource_base=e.alloc(96*RESOURCE_STRIDE)+RESOURCE_STRIDE;self.settings_base=e.alloc(96*SETTINGS_STRIDE)+SETTINGS_STRIDE
  self.maps={RESOURCE_TABLE:8,SETTINGS_TABLE:8,**{self.resource(i)+0x6188:8 for i in INDICES},**{self.settings(i)+0xdac:4 for i in INDICES}}
  if root_strings:
   self.maps={RESOURCE_TABLE:8,SETTINGS_TABLE:8,**{self.resource(i)+0x6178:32 for i in INDICES},**{self.settings(i)+0xdac:4 for i in INDICES}}
  self.publish()
 def resource(self,i):return self.resource_base+i*RESOURCE_STRIDE
 def settings(self,i):return self.settings_base+i*SETTINGS_STRIDE
 def publish(self):self.e.write_u64(RESOURCE_TABLE,self.resource_base);self.e.write_u64(SETTINGS_TABLE,self.settings_base)
 def prepare(self,case):
  # Record88 is the legacy root fixture's record. Other cells are distinct.
  available=self.e.read_u64(self.resource(88)+0x6188);mode=self.e.read_u32(self.settings(88)+0xdac);self.publish()
  for i in INDICES:
   if i==88:continue
   if self.root_strings:
    data=f'record{i}'.encode();raw=bytearray(32);raw[:len(data)]=data;raw[16:24]=(len(data) if available and (i+case)%3 else 0).to_bytes(8,'little');raw[24:32]=(15).to_bytes(8,'little');self.e.write(self.resource(i)+0x6178,raw)
   else:self.e.write_u64(self.resource(i)+0x6188,0 if (i+case)%3==0 else available)
   self.e.write_u32(self.settings(i)+0xdac,(7 if mode==0 else 0) if (i-88)&1 else mode)
