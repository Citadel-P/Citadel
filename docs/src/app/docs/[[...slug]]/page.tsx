import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { DocRedirect } from '@/components/doc-redirect';
import { legacyRoutes } from '@/lib/legacy-docs';
import { getDocsBasePath } from '../../../../site-config.mjs';

function redirectRule(slug: string[] | undefined) {
  return legacyRoutes[`/docs${slug?.length ? `/${slug.join('/')}` : ''}`];
}

export default async function Page(props: PageProps<'/docs/[[...slug]]'>) {
  const { slug } = await props.params;
  const rule = redirectRule(slug);
  if (!rule) notFound();
  return <main className="p-6"><DocRedirect rule={rule} basePath={getDocsBasePath()} /></main>;
}

export function generateStaticParams() {
  return Object.keys(legacyRoutes).map((route) => ({
    slug: route.slice('/docs'.length).split('/').filter(Boolean),
  }));
}

export async function generateMetadata(props: PageProps<'/docs/[[...slug]]'>): Promise<Metadata> {
  const { slug } = await props.params;
  const rule = redirectRule(slug);
  if (!rule) notFound();
  return {
    title: 'Page moved',
    alternates: { canonical: rule.to },
    robots: { index: false },
  };
}
