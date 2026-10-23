// Messages du moteur (codes stables) rendus dans la langue de l'interface.

import { t, type Key } from './i18n.svelte';
import { fmtLen, numTrim } from './format';
import { store } from './store.svelte';
import type { Issue } from './types';

/** Libellé du champ d'un message (`controllers.3.size.reach` → « Portée (taille) »). */
export function fieldLabel(field: string | undefined): string {
  if (!field) return '';
  const generic = field.replace(/^controllers\.\d+\./, 'controllers.');
  const key = `fields.${generic}` as Key;
  const text = t(key);
  return text === key ? field : text;
}

/** Champs exprimés en longueur (pour formater la valeur d'un message). */
const LENGTH_CODES = new Set(['plate.corner_limited', 'shape.corner_limited', 'plate.margin_too_large', 'holes.removed_small', 'bridge.thin']);

export function issueText(i: Issue): string {
  const value =
    i.value == null ? '' : LENGTH_CODES.has(i.code) ? fmtLen(i.value, store.units, 2) : numTrim(i.value, 2);
  const key = `issues.${i.code}` as Key;
  const text = t(key, { field: fieldLabel(i.field), value, count: i.count ?? '' });
  return text === key ? i.code : text;
}
