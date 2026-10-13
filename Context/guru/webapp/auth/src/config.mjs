import { betterAuth } from 'better-auth';
import pg from 'pg';

for (const key of ['DATABASE_URL', 'BETTER_AUTH_SECRET', 'VAULT_PUBLIC_URL', 'VAULT_INTERNAL_TOKEN']) {
  if (!process.env[key]) throw new Error(`Missing configuration: ${key}`);
}
export const publicURL = new URL(process.env.VAULT_PUBLIC_URL).origin;
export const allowedOrigins = new Set([publicURL, ...(process.env.VAULT_ALLOWED_ORIGINS || '').split(',').filter(Boolean)]);
export const pool = new pg.Pool({ connectionString: process.env.DATABASE_URL, max: 5, connectionTimeoutMillis: 5000, idleTimeoutMillis: 30000 });
export const auth = betterAuth({
  appName: 'Strategy Vault',
  baseURL: publicURL,
  secret: process.env.BETTER_AUTH_SECRET,
  trustedOrigins: [...allowedOrigins],
  database: pool,
  emailAndPassword: { enabled: true, minPasswordLength: 12, maxPasswordLength: 128, autoSignIn: false },
  user: { additionalFields: { role: { type: 'string', required: true, defaultValue: 'researcher', input: false } } },
  session: { expiresIn: 86400 * 7, updateAge: 86400, cookieCache: { enabled: false } },
  rateLimit: { enabled: true, window: 60, max: 40, customRules: { '/sign-in/email': { window: 60, max: 10 } } },
  advanced: { ipAddress: { ipAddressHeaders: ['x-vault-client-ip'] }, cookiePrefix: 'strategy-vault', useSecureCookies: publicURL.startsWith('https:') },
});
