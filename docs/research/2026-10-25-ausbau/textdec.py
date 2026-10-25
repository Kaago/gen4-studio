import struct, sys
def ch(c):
    if 0x121 <= c <= 0x12A: return chr(ord('0') + c - 0x121)
    if 0x12B <= c <= 0x144: return chr(ord('A') + c - 0x12B)
    if 0x145 <= c <= 0x15E: return chr(ord('a') + c - 0x145)
    if c == 0x1DE: return ' '
    if c == 0x1AE: return '.'
    if c == 0x1BE: return '-'
    return '?'
def decode(path):
    b = open(path, 'rb').read()
    n, seed = struct.unpack_from('<HH', b, 0)
    out = []
    for i in range(n):
        key = (765 * (i + 1) * seed) & 0xFFFF
        kl = key | (key << 16)
        off, ln = struct.unpack_from('<II', b, 4 + 8 * i)
        off ^= kl; ln ^= kl
        k2 = (0x91BD3 * (i + 1)) & 0xFFFF
        s = ''
        for j in range(ln):
            c = struct.unpack_from('<H', b, off + 2 * j)[0] ^ k2
            k2 = (k2 + 0x493D) & 0xFFFF
            if c == 0xFFFF: break
            s += ch(c)
        out.append(s)
    return out
for i, s in enumerate(decode(sys.argv[1])):
    print(f"{i:03}\t{s}")
