#!/usr/bin/env python3
"""Whole original refresh; shared controlled providers and deterministic cases."""
import sys
from ui_context_refresh_harness import harness,cases,prepare,GLOBAL,REFERENCE_SHA256

def main(path):
 h=harness(path);rows=cases()
 print('# reference_sha256',REFERENCE_SHA256)
 print('# complete original refresh; providers/CRT/free/notify controlled; exact branch/arithmetic/cleanup/list traversal')
 for i,row in enumerate(rows):
  fields=prepare(h,i,row);h.e.call(0x1411396d0,ints=[h.obj],max_instructions=100000)
  print('C',*fields,'->',h.e.read(h.obj,96).hex(),h.e.read_u64(GLOBAL),h.links(),'|'.join(h.events))
 print('# cases',len(rows))
if __name__=='__main__':main(sys.argv[1])
