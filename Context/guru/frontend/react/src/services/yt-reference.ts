/**
 * Reference files of any YT family (BTST, ATH, ADR): one lookup so the card dot, the reference box and the settings window do not
 * depend on the family. Each family has its own reference table (`btst-reference.ts`, `ath-reference.ts`).
 */
import {BtstReference, ReferenceFile, btstReference, mainTradesheet, referenceDefaults, universeReferenceDefaults} from './btst-reference';
import {athReference} from './ath-reference';
import {adrReference} from './adr-reference';

/** Brokerage the ATH pages open with: the AFLs' own default, 0.65% a side (AFL BROKERAGE), in bps. */
export const ATH_BROKERAGE_BPS = 65;

/** Brokerage the ADR pages open with: the team's ADR runs used 0.04% a side, in bps. */
export const ADR_BROKERAGE_BPS = 4;

/** The reference files of a BTST, ATH or ADR sub-variant, or null for any other strategy. */
export function ytFamilyReference(id: string | undefined): BtstReference | null {
  return btstReference(id) || athReference(id) || adrReference(id);
}

/** Name of the team's family folder that holds a sub-variant's reference files. */
export function ytFamilyFolder(id: string | undefined): string {
  return id?.startsWith('ytath_') ? 'ATH' : id?.startsWith('ytadr_') ? 'Advance Decline Ratio' : 'BTST';
}

/** A reference file's path as shown to the team: the family folder first ("BTST/…"), then its sub-folders and name. */
export function ytRefPath(id: string | undefined, path: string): string {
  const top = ytFamilyFolder(id);
  return path.startsWith(`${top}/`) ? path : `${top}/${path}`;
}

/** The settings a sub-variant page opens with: BTST's reference sheet settings, ATH: the engine defaults and the AFL brokerage. */
export function ytReferenceDefaults(id: string, ticker?: string): {params: Record<string, number>; cost: number; sheet: ReferenceFile | null} | null {
  const btst = referenceDefaults(id, ticker);
  if (btst) return btst;
  const ref = athReference(id) || adrReference(id);
  if (!ref) return null;
  const bps = id.startsWith('ytadr_') ? ADR_BROKERAGE_BPS : ATH_BROKERAGE_BPS;
  // the first sheet whose workbook states its settings when none is selected yet (the page then opens on those settings)
  const sheet = mainTradesheet(ref, ticker) || ref.tradesheets.find(t => !t.ticker && t.params && Object.keys(t.params).length) || null;
  return {params: {...(sheet?.params || {})}, cost: sheet?.cost ?? bps, sheet};
}

/**
 * The settings a stock-universe run opens with: BTST's stock trade sheet, or for ATH the first trade sheet whose workbook states its
 * settings (engine keys), with the AFL brokerage; none = the engine defaults.
 */
export function ytUniverseReferenceDefaults(id: string): {params: Record<string, number>; cost: number; sheet: ReferenceFile | null} | null {
  const btst = universeReferenceDefaults(id);
  if (btst) return btst;
  const ref = athReference(id);
  if (!ref) return null;
  const sheet = ref.tradesheets.find(t => !t.ticker && t.params && Object.keys(t.params).length) || null;
  return {params: {...(sheet?.params || {})}, cost: sheet?.cost ?? ATH_BROKERAGE_BPS, sheet};
}
