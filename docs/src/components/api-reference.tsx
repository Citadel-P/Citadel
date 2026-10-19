'use client';

import dynamic from 'next/dynamic';
import { useTheme } from 'next-themes';
import { useMemo } from 'react';
import { docsAssetUrl } from '@/lib/shared';
import type { RedocStandaloneProps } from 'redoc';

const Redoc = dynamic(() => import('redoc').then((module) => module.RedocStandalone), {
  ssr: false,
  loading: () => <p role="status" className="p-6 text-fd-muted-foreground">Loading API reference…</p>,
});

export function ApiReference() {
  const { resolvedTheme } = useTheme();
  const dark = resolvedTheme === 'dark';
  const options = useMemo<RedocStandaloneProps['options']>(() => ({
    hideDownloadButtons: true,
    nativeScrollbars: true,
    scrollYOffset: 64,
    theme: {
      spacing: { sectionHorizontal: 24, sectionVertical: 28 },
      colors: {
        primary: { main: dark ? '#93c5fd' : '#2563eb' },
        text: { primary: dark ? '#edf2f7' : '#0f172a', secondary: dark ? '#afbdcc' : '#475569' },
        gray: { 50: dark ? '#1d2a38' : '#f1f5f9', 100: dark ? '#17212d' : '#f8fafc' },
        border: { dark: dark ? '#334253' : '#dce3eb', light: dark ? '#17212d' : '#ffffff' },
      },
      typography: {
        fontSize: '14px',
        fontFamily: 'inherit',
        headings: { fontFamily: 'inherit' },
        code: { fontFamily: 'ui-monospace, monospace' },
      },
      sidebar: {
        width: '240px',
        backgroundColor: dark ? '#17212d' : '#f8fafc',
        textColor: dark ? '#afbdcc' : '#475569',
        activeTextColor: dark ? '#93c5fd' : '#2563eb',
      },
      rightPanel: { backgroundColor: '#17212d', textColor: '#edf2f7' },
      schema: {
        nestedBackground: dark ? '#1d2a38' : '#f1f5f9',
        arrow: { color: dark ? '#afbdcc' : '#475569' },
      },
      extensionsHook: (name: string) => name === 'UnderlinedHeader'
        ? `color: ${dark ? '#afbdcc' : '#475569'}; border-color: ${dark ? '#334253' : '#dce3eb'};`
        : '',
    },
  }), [dark]);

  return (
    <section aria-label="OpenAPI reference" className="not-prose min-w-0 w-full border-t border-fd-border">
      <div className="flex flex-wrap gap-4 border-b border-fd-border px-4 py-3 text-sm">
        <a href={docsAssetUrl('/api/openapi')} download="citadel-public-v1.json" className="text-fd-primary hover:underline">
          Download OpenAPI schema
        </a>
      </div>
      <Redoc specUrl={docsAssetUrl('/api/openapi')} options={options} />
    </section>
  );
}
