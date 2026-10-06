import { source } from '@/lib/source';
import { DocsBody, DocsDescription, DocsPage, DocsTitle } from 'fumadocs-ui/layouts/docs/page';
import { notFound } from 'next/navigation';
import { getMDXComponents } from '@/components/mdx';
import type { Metadata } from 'next';
import { createRelativeLink } from 'fumadocs-ui/mdx';
import { DocRedirect } from '@/components/doc-redirect';
import { ApiReference } from '@/components/api-reference';
import redirects from '../../../../redirects.json';
import { getDocsBasePath } from '../../../../site-config.mjs';

const legacyRoutes: Record<string, { to: string; anchors?: Record<string, string> }> = redirects;

function legacyRoute(slug: string[] | undefined) {
  return legacyRoutes[`/docs/${(slug ?? []).join('/')}`];
}

export default async function Page(props: PageProps<'/docs/[[...slug]]'>) {
  const params = await props.params;
  const redirect = legacyRoute(params.slug);
  if (redirect) return <DocRedirect rule={redirect} basePath={getDocsBasePath()} />;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;
  const content = (
    <MDX
      components={getMDXComponents({
        // Resolve links relative to the current documentation page.
        a: createRelativeLink(source, page),
      })}
    />
  );
  if (page.slugs.join('/') === 'reference/api') {
    return (
      <main className="min-w-0 w-full">
        <div className="max-w-5xl space-y-4 px-4 py-6 sm:px-6">
          <DocsTitle>{page.data.title}</DocsTitle>
          <DocsDescription className="mb-0">{page.data.description}</DocsDescription>
          <DocsBody>{content}</DocsBody>
        </div>
        <ApiReference />
      </main>
    );
  }
  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription className="mb-0">{page.data.description}</DocsDescription>
      <DocsBody>{content}</DocsBody>
    </DocsPage>
  );
}

export async function generateStaticParams() {
  return [
    ...source.generateParams(),
    ...Object.keys(legacyRoutes).map((route) => ({ slug: route.slice('/docs/'.length).split('/') })),
  ];
}

export async function generateMetadata(props: PageProps<'/docs/[[...slug]]'>): Promise<Metadata> {
  const params = await props.params;
  const redirect = legacyRoute(params.slug);
  if (redirect) return {
    title: 'Page moved',
    alternates: { canonical: redirect.to },
    robots: { index: false },
  };
  const page = source.getPage(params.slug);
  if (!page) notFound();

  return {
    title: page.data.title,
    description: page.data.description,
    alternates: { canonical: page.url },
  };
}
