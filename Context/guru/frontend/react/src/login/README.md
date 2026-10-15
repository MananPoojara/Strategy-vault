# Strategy Vault welcome and sign-in

The active implementation is React (`LoginPage.tsx`, `LoginForm.tsx`, `WelcomeScene.jsx`), adapted from the owner's supplied `Welcome-Page/` scene. The supplied Dadashri photograph, gold frame, flowers, incense and aarti lamp are preserved. The Young Turtle brand, demo username/signup flow, router and Motion dependency are replaced with Strategy Vault, real email authentication and native transitions.

The form posts only to the existing same-origin `/api/auth/sign-in/email`; existing cookie sessions, roles, rate limits and administrator-only account creation stay in the unchanged authentication service. There is no password-length rule added to sign-in and no browser credential storage. Same-origin return links preserve research hashes while external destinations, control/backslash forms and login loops are rejected.

`npm run build` builds the research app and then `dist-login/login.html`, `login.js`, `login.css`. The Dockerfile overlays these three generated files after the existing public files. The original public stylesheet is retained verbatim first because the administrator page shares it; new scene styling applies only to the login body/components. No gateway binary/path change is needed. Generated outputs are ignored by Git.

The repository builds a silent visual aarti by default. Optional audio must be an explicitly provided actual Ogg Opus file no larger than1MiB, kept outside Git:

```
VAULT_LOGIN_AUDIO=/absolute/private/path/aarti-ui-loop.ogg npm run build
```

For the container build, provide that private file as the BuildKit secret `login_audio`; its contents are copied only into the generated `dist/vendor/login/landing/aarti.ogg` asset. An absent optional secret produces the silent build. The reviewed deployment derivative is bounded to about120seconds; its original and output hashes/ffmpeg command belong in the private release receipt. The original long recording is never committed or copied into the image. No autoplay or initial audio request occurs. Aarti starts/stops by the explicit keyboard-operable control or welcome-scene click. Opening sign-in, hiding the tab or unmounting stops sound; returning to a tab does not restart it.

Lamp and smoke updates are capped at30frames/s. Smoke has the reference's fixed four strands, bounded particle lifetime and capped pixel density. Reduced-motion changes, hidden-tab state and unmount cleanups cancel their loops. Petals appear only while aarti is active and are unmounted after their fade.

No trading rules, source admissions, strategy calculations or results are part of this login change. Asset provenance/embedded font notices are under `frontend/vendor/login/`.
