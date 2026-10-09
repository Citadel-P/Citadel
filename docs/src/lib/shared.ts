export const appName = 'Citadel';
export { docsAssetUrl } from '../../site-config.mjs';
export const docsRoute = '/docs';
export const gitConfig = {
  user: 'Citadel-P',
  repo: 'Citadel',
  branch: 'main',
  // Keep license links pinned to the reviewed ELv2 terms adopted in PR #47.
  licenseRef: 'd42c63cf3a340078ede7530f07ab36d416a0f0f2',
};

const localSiteUrl = 'http://localhost:3000';

export function getSiteUrl() {
  const configuredUrl = process.env.NEXT_PUBLIC_DOCS_URL;
  if (configuredUrl) return configuredUrl.replace(/\/$/, '');
  if (process.env.CI === 'true') {
    throw new Error('NEXT_PUBLIC_DOCS_URL is required for CI documentation builds.');
  }
  return localSiteUrl;
}
