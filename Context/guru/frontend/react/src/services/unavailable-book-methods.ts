interface Definition {id: string; book: string; family?: string; family_name?: string; name?: string}
interface Reason {id: string; status: string; note?: string}
export interface UnavailableBookMethod {
  definition: Definition; key: string; name: string; status: string; reasons: string[];
}
/** Group parameters into their book method; an available sibling keeps the method available. */
export function unavailableBookMethods(definitions: Definition[], reasons: Reason[]): UnavailableBookMethod[] {
  const byId = new Map(reasons.map(row => [row.id, row]));
  const groups = new Map<string, Definition[]>();
  for (const def of definitions) {
    const key = `${def.book}:${def.family || def.id}`;
    const group = groups.get(key) || []; group.push(def); groups.set(key, group);
  }
  return [...groups.entries()].flatMap(([key, group]) => {
    if (group.some(def => byId.get(def.id)?.status === 'available')) return [];
    const states = group.map(def => byId.get(def.id));
    const noTrades = states.some(row => row?.status === 'no_trades');
    const notes = [...new Set(states.map(row => row?.note?.trim()).filter((note): note is string => !!note))];
    if (!notes.length || states.some(row => !row)) notes.push('No measured result or admission decision is available for this selection.');
    return [{key, definition: group[0], name: group[0].family_name || group[0].name || group[0].id,
      status: noTrades ? 'no_trades' : states.find(Boolean)?.status || 'no_eligible_results',
      reasons: notes,
    }];
  });
}
