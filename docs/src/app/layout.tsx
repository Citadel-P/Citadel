import { Provider } from '@/components/provider';
import { Analytics } from '@/components/analytics/analytics';
import { source } from '@/lib/source';
import { docsAssetUrl, getSiteUrl } from '@/lib/shared';
import type { Metadata } from 'next';
import './global.css';

export const metadata: Metadata = {
  metadataBase: new URL(getSiteUrl()),
  title: {
    default: 'Citadel Documentation',
    template: '%s | Citadel Documentation',
  },
  description: 'Install, configure, operate, and secure Citadel.',
  icons: [{ rel: 'icon', url: docsAssetUrl('/favicon.svg'), type: 'image/svg+xml' }],
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex flex-col min-h-screen">
        <Analytics
          siteUrl={getSiteUrl()}
          routes={Object.fromEntries(
            source.getPages().map((page) => [page.url.replace(/\/$/, '') || '/', page.data.title]),
          )}
        >
          <Provider>{children}</Provider>
        </Analytics>
      </body>
    </html>
  );
}
