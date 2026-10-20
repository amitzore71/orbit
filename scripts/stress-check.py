"""Exercise current native widget creation/destruction using an isolated database."""
import argparse
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--exe',type=Path,default=root/'target/release/orbit.exe')
p.add_argument('--cycles',type=int,default=50)
args=p.parse_args()
subprocess.run(['node',str(root/'frontend/tests/native-ui.mjs'),str(args.exe.resolve())],cwd=root,env={**os.environ,'ORBIT_WIDGET_CYCLES':str(args.cycles)},check=True)
