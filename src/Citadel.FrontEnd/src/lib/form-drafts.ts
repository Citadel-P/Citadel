import { useContext } from 'react';
import { jwtDecode } from 'jwt-decode';
import { AuthContext } from '@/features/auth/auth-context';
import { ApiClientContext } from '@/api/api-client-context';

export function scopedDraftKey(key: string | undefined, token: string | undefined, server: string): string | undefined {
  if (!key || !token) return undefined;
  try {
    const { sub, iss } = jwtDecode<{ sub?: string; iss?: string }>(token);
    if (!sub) return undefined;
    return `citadel:form-draft:${encodeURIComponent(JSON.stringify([server, iss ?? '', sub, key]))}`;
  } catch {
    return undefined;
  }
}

export function useFormDraftKey(key?: string) {
  const auth = useContext(AuthContext);
  const api = useContext(ApiClientContext);
  return scopedDraftKey(key, auth?.isAuthenticated ? auth.accessToken : undefined, api?.apiClient.baseUrl ?? '');
}

// Match credential values, not references such as secretId, registryId or tokenExpiration.
function isSensitiveKey(key: string): boolean {
  const normalized = key.replace(/[^a-z0-9]/gi, '').toLowerCase();
  return (
    /(?:password|passwd|passphrase|secret|token|apikey|privatekey|secretkey|accesskey|authorization|credential|credentials)$/.test(
      normalized,
    ) || normalized === 'pat'
  );
}

function isReference(value: unknown): boolean {
  return typeof value === 'string' && /^\$\{[A-Za-z_][A-Za-z0-9_.-]*\}$/.test(value.trim());
}

// Editors remain draftable. Detect common embedded credentials without attempting
// to parse or rewrite arbitrary scripts/Compose files; use stored secret references there.
function containsCredential(value: string): boolean {
  if (/-----BEGIN [A-Z ]*PRIVATE KEY-----/.test(value)) return true;
  if (/[a-z][a-z0-9+.-]*:\/\/[^\s/@]+:[^\s/@]+@/i.test(value)) return true;
  for (const match of value.matchAll(/(?=(?:^|[\s{,;?&])['"]?([\w.-]+)['"]?\s*[:=]\s*['"]?([^\s'",;&]+))/g)) {
    if (isSensitiveKey(match[1]) && !isReference(match[2])) return true;
  }
  return false;
}

/** Preserve form settings by default. Explicit exclusions cover opaque secret fields. */
export function sanitizeDraft<T>(update: Partial<T>, excludedPaths: readonly string[] = []): Partial<T> {
  function visit(value: unknown, path: string): unknown {
    if (excludedPaths.includes(path)) return undefined;
    if (typeof value === 'string') return containsCredential(value) ? undefined : value;
    if (value === null || typeof value === 'number' || typeof value === 'boolean') return value;
    if (Array.isArray(value)) {
      return value.map((entry, index) => visit(entry, `${path}.${index}`)).filter((entry) => entry !== undefined);
    }
    if (!value || typeof value !== 'object') return undefined;

    const record = value as Record<string, unknown>;
    // Key/value editors (labels, environment variables, build arguments).
    const entryKey = typeof record.key === 'string' ? record.key : record.name;
    const sensitiveEntry = typeof entryKey === 'string' && isSensitiveKey(entryKey);
    const result: Record<string, unknown> = {};
    for (const [key, entry] of Object.entries(record)) {
      if (['__proto__', 'prototype', 'constructor'].includes(key)) continue;
      if (isSensitiveKey(key) && typeof entry !== 'boolean' && !isReference(entry)) continue;
      if (key === 'value' && sensitiveEntry && !isReference(entry)) continue;
      const sanitized = visit(entry, path ? `${path}.${key}` : key);
      if (sanitized !== undefined) result[key] = sanitized;
    }
    // Do not manufacture dirty fields when an object contained only credentials.
    return Object.keys(result).length || !Object.keys(record).length ? result : undefined;
  }
  return (visit(update, '') ?? {}) as Partial<T>;
}

/** Presentation only: never submit the masked object as a form value. */
export function maskFormSecrets(value: unknown, excludedPaths: readonly string[] = []): unknown {
  function visit(entry: unknown, path: string, sensitive = false): unknown {
    if (entry === null || entry === undefined || typeof entry === 'boolean') return entry;
    if (sensitive || excludedPaths.includes(path)) return entry === '' ? '' : '******';
    if (typeof entry === 'string') return containsCredential(entry) ? '******' : entry;
    if (Array.isArray(entry)) return entry.map((item, index) => visit(item, `${path}.${index}`));
    if (typeof entry !== 'object') return entry;
    const record = entry as Record<string, unknown>;
    const name = typeof record.key === 'string' ? record.key : record.name;
    return Object.fromEntries(
      Object.entries(record).map(([key, item]) => [
        key,
        visit(
          item,
          path ? `${path}.${key}` : key,
          isSensitiveKey(key) || (key === 'value' && typeof name === 'string' && isSensitiveKey(name)),
        ),
      ]),
    );
  }
  return visit(value, '');
}
