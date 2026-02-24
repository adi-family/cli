export function generateCid(): string {
  if (typeof crypto.randomUUID === 'function') return crypto.randomUUID();
  return Array.from(crypto.getRandomValues(new Uint8Array(16)))
    .map((b, i) => ([4, 6, 8, 10].includes(i) ? '-' : '') + (i === 6 ? ((b & 0x0f) | 0x40).toString(16) : i === 8 ? ((b & 0x3f) | 0x80).toString(16) : b.toString(16).padStart(2, '0')))
    .join('');
}
