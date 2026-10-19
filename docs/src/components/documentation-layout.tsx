'use client';

import { DocsLayout, type DocsLayoutProps } from 'fumadocs-ui/layouts/docs';
import type { MainItemType } from 'fumadocs-ui/layouts/shared';
import { ThemeSwitch } from 'fumadocs-ui/layouts/shared/slots/theme-switch';
import { ArrowLeft, ArrowUpRight } from 'lucide-react';
import Image from 'next/image';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { appName, docsAssetUrl } from '@/lib/shared';

export function DocumentationLayout({ children, ...props }: DocsLayoutProps) {
  const pathname = usePathname();
  if (pathname.replace(/\/$/, '') !== '/docs/reference/api') {
    const utilityLinks = (props.links ?? []).filter(
      (item): item is MainItemType => !item.type || item.type === 'main',
    );

    return (
      <DocsLayout
        {...props}
        containerProps={{
          ...props.containerProps,
          style: {
            ...props.containerProps?.style,
            gridTemplate: `"sidebar sidebar header toc toc" auto
              "sidebar sidebar toc-popover toc toc" auto
              "sidebar sidebar main toc toc" 1fr / 0px var(--fd-sidebar-col)
              minmax(0, 1fr) var(--fd-toc-width) 0px`,
          },
        }}
        links={(props.links ?? []).filter((item) => item.type && item.type !== 'main')}
        sidebar={{
          ...props.sidebar,
          banner: <p className="docs-sidebar-label">Documentation</p>,
          footer: (
            <nav aria-label="Documentation resources" className="docs-sidebar-resources">
              {utilityLinks.map((item) => (
                <Link
                  key={item.url}
                  href={item.url}
                  aria-current={pathname.replace(/\/$/, '') === item.url ? 'page' : undefined}
                  target={item.external ? '_blank' : undefined}
                  rel={item.external ? 'noopener noreferrer' : undefined}
                >
                  {item.text}
                  {item.external && <ArrowUpRight aria-hidden="true" />}
                </Link>
              ))}
            </nav>
          ),
        }}
      >
        {children}
      </DocsLayout>
    );
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
