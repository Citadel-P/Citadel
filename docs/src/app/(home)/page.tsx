import Image from 'next/image';
import Link from 'next/link';
import { ArrowRight, Boxes, Rocket, Workflow } from 'lucide-react';
import { docsAssetUrl } from '@/lib/shared';

export default function HomePage() {
  return (
    <main className="mx-auto flex w-full max-w-6xl flex-1 flex-col px-6 py-16 md:py-24">
      <section className="grid items-center gap-12 md:grid-cols-[1fr_280px]">
        <div>
          <p className="mb-4 text-sm font-semibold text-fd-primary">Citadel documentation</p>
          <h1 className="max-w-3xl text-4xl font-semibold tracking-tight md:text-6xl">
            Run and manage Docker from one place.
          </h1>
          <p className="mt-6 max-w-2xl text-lg text-fd-muted-foreground">
            Citadel gives you a web interface for your containers, Compose stacks, and Docker
            Swarms. Start locally, then connect more hosts when you need them.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link
              href="/docs/getting-started/quick-start"
              className="inline-flex items-center gap-2 rounded-md bg-fd-primary px-4 py-2.5 font-medium text-fd-primary-foreground"
            >
              Quick start <ArrowRight className="size-4" aria-hidden="true" />
            </Link>
            <Link
              href="/docs"
              className="rounded-md border border-fd-border px-4 py-2.5 font-medium hover:bg-fd-accent"
            >
              What is Citadel?
            </Link>
          </div>
        </div>
        <Image className="mx-auto size-56" src={docsAssetUrl('/logo.svg')} alt="Citadel fortress logo" width={224} height={224} priority />
      </section>

      <section className="mt-20 grid gap-4 md:grid-cols-3" aria-label="Documentation areas">
        <Feature href="/docs/getting-started/quick-start" icon={<Rocket />} title="Start in minutes">
          Run Citadel locally, create your account, and connect Docker.
        </Feature>
        <Feature href="/docs/guides/deployments" icon={<Boxes />} title="Deploy workloads">
          Create containers, Compose stacks, and native Swarm services.
        </Feature>
        <Feature href="/docs/guides/automation-actions" icon={<Workflow />} title="Automate operations">
          Schedule updates, backups, alerts, builds, and repeatable actions.
        </Feature>
      </section>
    </main>
  );
}

function Feature({
  href,
  icon,
  title,
  children,
}: {
  href: string;
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <Link href={href} className="rounded-lg border border-fd-border p-5 transition-colors hover:bg-fd-accent">
      <span className="mb-4 block size-5 text-fd-primary" aria-hidden="true">{icon}</span>
      <h2 className="font-semibold">{title}</h2>
      <p className="mt-2 text-sm text-fd-muted-foreground">{children}</p>
    </Link>
  );
}
