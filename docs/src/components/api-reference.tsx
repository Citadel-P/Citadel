'use client';

import dynamic from 'next/dynamic';
import { useEffect } from 'react';
import { useTheme } from 'next-themes';
import { docsAssetUrl } from '@/lib/shared';

const Redoc = dynamic(async () => {
  const [{ RedocStandalone }, { HashRouter }] = await Promise.all([
    import('redoc'),
    import('react-router'),
  ]);
  return function ApiViewer() {
    return (
      <HashRouter>
        <RedocStandalone
          specUrl={docsAssetUrl('/api/openapi')}
          options={{ hideDownloadButtons: true, scrollYOffset: 64, disableTelemetry: true }}
        />
      </HashRouter>
    );
  };
}, {
  ssr: false,
  loading: () => <p role="status" className="p-6 text-fd-muted-foreground">Loading API reference…</p>,
});

export function ApiReference() {
  const { resolvedTheme } = useTheme();
  useEffect(() => {
    if (!resolvedTheme) return;
    const root = document.documentElement;
    // ReDoc 3 writes its own theme to <html>. The docs theme provider remains
    // authoritative, including when ReDoc updates its internal preferences.
    function syncTheme() {
      const dark = resolvedTheme === 'dark';
      if (root.classList.contains('dark') !== dark) root.classList.toggle('dark', dark);
      if (root.classList.contains('light') === dark) root.classList.toggle('light', !dark);
    }
    const observer = new MutationObserver(syncTheme);
    observer.observe(root, { attributes: true, attributeFilter: ['class'] });
    syncTheme();
    return () => observer.disconnect();
  }, [resolvedTheme]);

  return (
    <section aria-label="OpenAPI reference" className="docs-api-reference not-prose min-w-0 w-full border-t border-fd-border">
      <div className="flex flex-wrap gap-4 border-b border-fd-border px-4 py-3 text-sm">
        <a href={docsAssetUrl('/api/openapi')} download="citadel-public-v1.json" className="text-fd-primary hover:underline">
          Download OpenAPI schema
        </a>
      </div>
      <Redoc />
    </section>
  );
}
