import type { LabMode } from "./simple";

/** The view the page opens on: always Simple (user, 23-11-2026); Advanced lasts until the page is left or reloaded.
 *  The test setup opens on Advanced, which most page tests cover. A module of its own (no imports at run time), so the
 *  test setup can set it without loading the lab before a test file mocks its client. */
export const OPENING: { mode: LabMode } = { mode: "simple" };
