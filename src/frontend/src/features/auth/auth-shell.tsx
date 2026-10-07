import type { ReactNode } from 'react';
import LogoIcon from '@/assets/logo.svg?react';
import { cn } from '@/lib/utils';
import { sourceAndLicenseLinks } from '@/lib/source-license';

export function AuthShell({
  title,
  description,
  children,
  panelClassName,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  panelClassName?: string;
}) {
  const { sourceUrl, licenseUrl } = sourceAndLicenseLinks();
  return (
    <main className="flex min-h-svh w-full items-center justify-center bg-muted/20 px-4 py-10 sm:px-6">
      <div className={cn('w-full max-w-[26rem]', panelClassName)}>
        <div className="mb-8 flex items-center justify-center gap-2.5">
          <LogoIcon aria-hidden="true" className="size-8 shrink-0" />
          <span className="text-xl font-semibold tracking-tight">Citadel</span>
        </div>
        <section className="rounded-xl border bg-card p-6 shadow-sm sm:p-8" aria-labelledby="auth-title">
          <div className="mb-6 space-y-2">
            <h1 id="auth-title" className="text-xl font-semibold tracking-tight">
              {title}
            </h1>
            {description && <p className="text-sm leading-relaxed text-muted-foreground">{description}</p>}
          </div>
          <div className="space-y-5">{children}</div>
        </section>
        <footer className="mt-4 space-y-1 text-center text-xs leading-relaxed text-muted-foreground">
          <p>© Citadel authors and contributors. AGPL-3.0; no warranty.</p>
          <p>You may copy, modify and redistribute Citadel under its license.</p>
          <nav aria-label="Source and license" className="flex justify-center gap-4">
            <a href={sourceUrl} target="_blank" rel="noopener noreferrer" className="underline-offset-4 hover:underline">
              Source code
            </a>
            <a href={licenseUrl} target="_blank" rel="noopener noreferrer" className="underline-offset-4 hover:underline">
              License terms
            </a>
          </nav>
        </footer>
      </div>
    </main>
  );
}
