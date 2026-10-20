"""Capture the current web interface at supported window sizes, themes, and DPI scales."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
runner='pnpm.cmd' if os.name=='nt' else 'pnpm'
subprocess.run([runner,'test:e2e','--grep','visual states'],cwd=root/'frontend',check=True)
