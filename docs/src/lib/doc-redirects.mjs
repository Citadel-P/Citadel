/**
 * Resolve legacy documentation URLs, including sections moved to another page.
 * @param {{ to: string, anchors?: Record<string, string> }} rule
 * @param {string} [hash]
 * @param {string} [search]
 * @param {string} [basePath]
 */
export function redirectDestination(rule, hash = '', search = '', basePath = '') {
  const target = rule.anchors?.[hash.replace(/^#/, '')] ?? `${rule.to}${hash}`;
  const [pathname, anchor] = target.split('#', 2);
  return `${basePath}${pathname.replace(/\/$/, '')}/${search}${anchor ? `#${anchor}` : ''}`;
}

/**
 * Export an old /docs URL for every canonical page, retaining older moves.
 * @param {string[]} pageUrls
 * @param {Record<string, { to: string, anchors?: Record<string, string> }>} movedPages
 * @returns {Record<string, { to: string, anchors?: Record<string, string> }>}
 */
export function legacyDocRedirects(pageUrls, movedPages) {
  return {
    ...Object.fromEntries(pageUrls.map((url) => {
      const route = url.replace(/\/$/, '') || '/';
      return [route === '/' ? '/docs' : `/docs${route}`, { to: route }];
    })),
    ...movedPages,
  };
}
