/**
 * Derive the export prefix from the published URL. A project Pages site lives
 * under a path such as /Citadel; a custom-domain or local site uses the root.
 * @param {string} [siteUrl]
 */
export function getDocsBasePath(siteUrl = process.env.NEXT_PUBLIC_DOCS_URL || 'http://localhost:3000') {
  const url = new URL(siteUrl);
  if (!['http:', 'https:'].includes(url.protocol)) {
    throw new Error('NEXT_PUBLIC_DOCS_URL must be an HTTP or HTTPS URL.');
  }
  return url.pathname.replace(/\/+$/, '');
}

/** @param {string} assetPath */
export function docsAssetUrl(assetPath) {
  return `${getDocsBasePath()}${assetPath}`;
}
