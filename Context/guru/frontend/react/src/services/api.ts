export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}
/** Keep credentials in the existing HttpOnly cookie; never copy tokens into storage. */
export async function apiJson<T = unknown>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  if (!path.startsWith("/api/"))
    throw new Error("Only same-origin application API requests are supported.");
  const response = await fetch(path, {
    ...init,
    credentials: "same-origin",
    headers: { Accept: "application/json", ...init.headers },
  });
  if (!response.ok) {
    let message = `Request failed (${response.status}).`;
    try {
      const body = await response.json();
      if (typeof body.error === "string") message = body.error;
    } catch {
      /* Non-JSON upstream error: preserve the status. */
    }
    throw new ApiError(message, response.status);
  }
  return response.json() as Promise<T>;
}
export interface Account {
  user: {
    id: string;
    name: string;
    email: string;
    role: "admin" | "researcher" | "viewer";
  };
  settings: { theme: string; lastView?: string };
}
export const getAccount = () => apiJson<Account>("/api/app/me");
export const saveSettings = (settings: Partial<Account["settings"]>) =>
  apiJson("/api/app/settings", {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(settings),
  });
export const signOut = () =>
  apiJson("/api/auth/sign-out", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: "{}",
  });
