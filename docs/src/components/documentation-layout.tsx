'use client';

import { DocsLayout, type DocsLayoutProps } from 'fumadocs-ui/layouts/docs';
import { ThemeSwitch } from 'fumadocs-ui/layouts/shared/slots/theme-switch';
import { ArrowLeft } from 'lucide-react';
import Image from 'next/image';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { appName, docsAssetUrl } from '@/lib/shared';

export function DocumentationLayout({ children, ...props }: DocsLayoutProps) {
  const pathname = usePathname();
  if (pathname.replace(/\/$/, '') !== '/docs/reference/api') {
    return <DocsLayout {...props}>{children}</DocsLayout>;
  }

  return (
    <div className="min-w-0 w-full">
      <header className="sticky top-0 z-50 flex h-16 items-center justify-between gap-4 border-b border-fd-border bg-fd-background px-4 sm:px-6">
        <Link href="/" className="flex items-center gap-2 font-semibold">
          <Image src={docsAssetUrl('/logo.svg')} alt="" width={24} height={24} aria-hidden="true" />
          {appName}
        </Link>
        <nav aria-label="API reference navigation" className="flex items-center gap-4">
          <Link href="/docs" className="flex items-center gap-2 text-sm text-fd-muted-foreground hover:text-fd-foreground">
            <ArrowLeft aria-hidden="true" className="size-4" />
            Back to docs
          </Link>
          <ThemeSwitch />
        </nav>
      </header>
      {children}
    </div>
  );
}
