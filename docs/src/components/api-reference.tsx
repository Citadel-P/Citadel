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
        primary: { main: dark ? '#60a5fa' : '#1673cc' },
        text: { primary: dark ? '#e5e5e5' : '#262626', secondary: dark ? '#a3a3a3' : '#525252' },
        gray: { 50: dark ? '#202020' : '#fafafa', 100: dark ? '#262626' : '#f5f5f5' },
        border: { dark: dark ? '#404040' : '#d4d4d4', light: dark ? '#262626' : '#ffffff' },
      },
      typography: {
        fontSize: '14px',
        fontFamily: 'inherit',
        headings: { fontFamily: 'inherit' },
        code: { fontFamily: 'ui-monospace, monospace' },
      },
      sidebar: {
        width: '240px',
        backgroundColor: dark ? '#171717' : '#fafafa',
        textColor: dark ? '#d4d4d4' : '#404040',
        activeTextColor: dark ? '#60a5fa' : '#1673cc',
      },
      rightPanel: { backgroundColor: '#171717', textColor: '#e5e5e5' },
      schema: {
        nestedBackground: dark ? '#202020' : '#fafafa',
        arrow: { color: dark ? '#a3a3a3' : '#525252' },
      },
      extensionsHook: (name: string) => name === 'UnderlinedHeader'
        ? `color: ${dark ? '#a3a3a3' : '#525252'}; border-color: ${dark ? '#404040' : '#d4d4d4'};`
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
