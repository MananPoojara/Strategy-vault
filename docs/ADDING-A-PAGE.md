# Add a React page

Create your own branch from `Manan`. Work in `Context/guru/frontend/react`, not the historical HTML files.

1. Create `src/pages/YourPage.tsx` exporting a React component.
2. Import it in `src/app/custom-pages.ts` and register it:

```tsx
import {YourPage} from '../pages/YourPage';
export const customPages:PageRegistration[]=[
  {path:'/your-page',label:'Your page',component:YourPage,showInNavigation:true},
];
```

The URL is `/#/your-page`. Existing shared/saved links continue to use hash routing. Do not replace reserved routes `/`, `/guru/...`, `/s/...`, `/o/...`, `/source/...`, `/strategies` or `/saved`.

Use `apiJson` from `src/services/api.ts` for authenticated `/api/...` requests. The existing HttpOnly cookie provides the session. New backend write endpoints still require server-side authorization and origin validation. Never put credentials into frontend variables or localStorage.

Use the existing CSS variables (`--bg`, `--panel`, `--card`, `--ink`, `--blue`, `--green`, `--red`) to support all themes. Distinguish loading, unavailable-data and measured results. Do not fabricate data or performance to populate a page.

Run `npm run build` and `npm run test:coverage`, add meaningful tests for the new behaviour, and verify desktop/mobile in a browser. Push your own branch and open a pull request for review. Do not push main.
