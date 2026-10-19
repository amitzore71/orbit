"""Create a small code-drawn, multi-resolution ICO without image dependencies."""
import math
import pathlib
import struct

images = []
for size in (16, 32, 48, 64):
    pixels = bytearray()
    for y in range(size - 1, -1, -1):
        for x in range(size):
            alpha = 0
            for sy in (0.25, 0.75):
                for sx in (0.25, 0.75):
                    px, py = (x + sx) / size - 0.5, (y + sy) / size - 0.5
                    distance = math.hypot(px, py)
                    if 0.29 < distance < 0.44 or (abs(px) < 0.045 and -0.22 < py < 0.025):
                        alpha += 255 / 4
            pixels.extend((53, 71, 183, round(alpha)))
    mask = bytes(((size + 31) // 32) * 4 * size)
    bitmap = struct.pack('<IiiHHIIiiII', 40, size, size * 2, 1, 32, 0, len(pixels), 0, 0, 0, 0) + pixels + mask
    images.append((size, bitmap))
offset = 6 + 16 * len(images)
entries, data = bytearray(), bytearray()
for size, bitmap in images:
    entries.extend(struct.pack('<BBBBHHII', size, size, 0, 0, 1, 32, len(bitmap), offset))
    data.extend(bitmap)
    offset += len(bitmap)
output = pathlib.Path(__file__).resolve().parent.parent / 'crates/app/assets/orbit.ico'
output.parent.mkdir(parents=True, exist_ok=True)
output.write_bytes(struct.pack('<HHH', 0, 1, len(images)) + entries + data)
print(output)
