import { Provider } from '@/components/provider';
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
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
