/** Drops the "[Declared reading(s)…]" block (it runs to the end of a strategy note); text before it is kept. */
export const stripDeclaredReadings = (notes?: string | null): string => String(notes ?? '').replace(/\s*\[Declared readings?[^\]]*\][\s\S]*$/, '').trim();
