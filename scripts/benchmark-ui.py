"""Measure native UI feedback and frame intervals through the real WebView2 bridge."""
from pathlib import Path
import subprocess
import sys
root=Path(__file__).resolve().parents[1]
subprocess.run(['node',str(root/'frontend/tests/native-ui.mjs'),*sys.argv[1:]],cwd=root,check=True)
