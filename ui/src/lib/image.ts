// Images des contrôleurs Image : conversion d'un fichier en luminance
// réduite (au plus MAX_SIDE pixels de côté, stockée en base64 dans le
// projet) et rendu à l'écran (canevas en niveaux de gris, mis en cache).

import type { Raster } from './types';

/** Plus grand côté d'une image de contrôleur (pixels) : assez pour un motif
 *  de quelques centaines de perforations, léger dans le fichier de projet. */
export const MAX_SIDE = 200;

export async function rasterFromFile(file: File): Promise<Raster> {
  const bmp = await createImageBitmap(file);
  const k = Math.min(1, MAX_SIDE / Math.max(bmp.width, bmp.height));
  const w = Math.max(1, Math.round(bmp.width * k));
  const h = Math.max(1, Math.round(bmp.height * k));
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const ctx = c.getContext('2d')!;
  // Les zones transparentes comptent comme du blanc (aucune influence).
  ctx.fillStyle = '#fff';
  ctx.fillRect(0, 0, w, h);
  ctx.drawImage(bmp, 0, 0, w, h);
  bmp.close();
  const px = ctx.getImageData(0, 0, w, h).data;
  const luma = new Uint8Array(w * h);
  for (let i = 0; i < w * h; i++) {
    luma[i] = Math.round(0.299 * px[4 * i] + 0.587 * px[4 * i + 1] + 0.114 * px[4 * i + 2]);
  }
  let bin = '';
  for (let i = 0; i < luma.length; i += 0x8000) bin += String.fromCharCode(...luma.subarray(i, i + 0x8000));
  return { width: w, height: h, luma: btoa(bin) };
}

const cache = new Map<string, HTMLCanvasElement>();

/** Canevas en niveaux de gris d'une image de contrôleur. */
export function rasterCanvas(r: Raster): HTMLCanvasElement | null {
  const hit = cache.get(r.luma);
  if (hit) return hit;
  let bin: string;
  try {
    bin = atob(r.luma);
  } catch {
    return null;
  }
  if (bin.length !== r.width * r.height) return null;
  const c = document.createElement('canvas');
  c.width = r.width;
  c.height = r.height;
  const ctx = c.getContext('2d')!;
  const img = ctx.createImageData(r.width, r.height);
  for (let i = 0; i < bin.length; i++) {
    const v = bin.charCodeAt(i);
    img.data[4 * i] = v;
    img.data[4 * i + 1] = v;
    img.data[4 * i + 2] = v;
    img.data[4 * i + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  if (cache.size > 16) cache.clear();
  cache.set(r.luma, c);
  return c;
}
