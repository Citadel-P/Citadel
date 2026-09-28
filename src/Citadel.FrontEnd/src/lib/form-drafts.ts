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

// Only explicitly opted-in scalar fields are persisted. Objects, arrays, scripts,
// environment maps and credentials remain in memory unless individually audited.
export function pickDraftFields<T>(update: Partial<T>, paths: readonly string[]): Partial<T> {
  const result: Record<string, unknown> = {};
  for (const path of paths) {
    const keys = path.split('.');
    if (keys.some((key) => ['__proto__', 'prototype', 'constructor'].includes(key))) continue;
    let value: unknown = update;
    for (const key of keys) {
      if (!value || typeof value !== 'object' || !Object.hasOwn(value, key)) {
        value = undefined;
        break;
      }
      value = (value as Record<string, unknown>)[key];
    }
    if (value !== null && !['string', 'number', 'boolean'].includes(typeof value)) continue;
    let target = result;
    for (const key of keys.slice(0, -1)) {
      target[key] ??= {};
      target = target[key] as Record<string, unknown>;
    }
    target[keys[keys.length - 1]] = value;
  }
  return result as Partial<T>;
}
