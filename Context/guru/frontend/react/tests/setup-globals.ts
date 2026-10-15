// index.html loads /joe-ross-evidence.js before the app module, and src/compat/catalogue.js reads it at import; jsdom has no such script.
(window as unknown as { JoeRossEvidence?: unknown }).JoeRossEvidence ??= { books: [] };
