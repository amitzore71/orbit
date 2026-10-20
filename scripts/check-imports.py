"""Resolve a PE executable's imported functions against the local Windows DLLs."""
import ctypes
from pathlib import Path
import struct
import sys
data=Path(sys.argv[1]).read_bytes()
pe=struct.unpack_from('<I',data,0x3c)[0];opt=pe+24;table=opt+struct.unpack_from('<H',data,pe+20)[0]
sections=[struct.unpack_from('<IIII',data,table+i*40+8) for i in range(struct.unpack_from('<H',data,pe+6)[0])]
def offset(rva):
    return next(raw+rva-start for size,start,length,raw in sections if start<=rva<start+max(size,length))
def string(pos):return data[pos:data.index(0,pos)].decode('ascii')
kernel=ctypes.WinDLL('kernel32',use_last_error=True)
kernel.LoadLibraryW.argtypes=[ctypes.c_wchar_p];kernel.LoadLibraryW.restype=ctypes.c_void_p
kernel.GetProcAddress.argtypes=[ctypes.c_void_p,ctypes.c_char_p];kernel.GetProcAddress.restype=ctypes.c_void_p
cursor=offset(struct.unpack_from('<I',data,opt+120)[0]);missing=[]
while any(data[cursor:cursor+20]):
    original,_,_,name,first=struct.unpack_from('<IIIII',data,cursor);dll=string(offset(name));lib=kernel.LoadLibraryW(dll)
    if not lib:missing.append((dll,'DLL missing',ctypes.get_last_error()));cursor+=20;continue
    thunk=offset(original or first);count=0
    while (entry:=struct.unpack_from('<Q',data,thunk)[0]):
        if not entry>>63:
            symbol=string(offset(entry)+2)
            if not kernel.GetProcAddress(lib,symbol.encode()):missing.append((dll,symbol))
        count+=1;thunk+=8
    print(dll,count)
    cursor+=20
print('Missing:',missing)
sys.exit(bool(missing))
