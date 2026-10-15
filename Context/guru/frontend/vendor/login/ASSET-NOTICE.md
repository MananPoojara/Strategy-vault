# Supplied welcome-scene assets

The Dadashri photograph, gold-frame artwork, lamp images, flower images and font files were supplied by the project owner in `Welcome-Page/`. They are copied without image alteration. This notice records their provenance; it does not invent an artwork/music licence or claim ownership of the supplied originals. Existing copyright and asset rights remain with their respective holders.

The exact embedded font copyright/licence metadata is retained in `FONT-NOTICES.json`. The supplied font files identify SIL Open Font License1.1 (or its licence URL); the standard OFL1.1 text accompanies them. Font binaries are unchanged. No font or photograph is downloaded at runtime.

The source scene is adapted in `frontend/react/src/login/`. Motion/router dependencies from the demonstration are not shipped: existing React plus native CSS transitions handle this integration. The application's existing dependency notices continue to apply.

Audio is optional and is not committed to Git. An explicitly supplied, reviewed Ogg Opus UI loop may be embedded at build time. Without it, the aarti remains visual and the control says “visual”; the page neither requests a missing recording nor pretends to play music. Any included recording retains the supplied source's separate rights; this notice grants none.
