// index.html loads /joe-ross-evidence.js before the app module, and src/compat/catalogue.js reads it at import; jsdom has no such script.
(window as unknown as { JoeRossEvidence?: unknown }).JoeRossEvidence ??= { books: [] };

// The page opens in the Simple view by default; existing page tests cover the full (Advanced) editor. tests/lab-simple.test.tsx clears this.
try { localStorage.setItem("lab.mode", "advanced"); } catch { /* no storage */ }
