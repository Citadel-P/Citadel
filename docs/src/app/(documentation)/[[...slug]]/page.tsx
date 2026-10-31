import { source } from '@/lib/source';
import { DocsBody, DocsDescription, DocsPage, DocsTitle } from 'fumadocs-ui/layouts/docs/page';
import { notFound } from 'next/navigation';
import { getMDXComponents } from '@/components/mdx';
import type { Metadata } from 'next';
import { createRelativeLink } from 'fumadocs-ui/mdx';
import { ApiReference } from '@/components/api-reference';
import { PageHelpfulness } from '@/components/analytics/analytics';
import { HomeFragmentRedirect } from '@/components/doc-redirect';
import { getDocsBasePath } from '../../../../site-config.mjs';

export default async function Page(props: PageProps<'/[[...slug]]'>) {
  const params = await props.params;
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
        <div className="px-4 sm:px-6"><PageHelpfulness path={page.url} /></div>
      </main>
    );
  }
  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      {page.slugs.length === 0 && <HomeFragmentRedirect basePath={getDocsBasePath()} />}
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription className="mb-0">{page.data.description}</DocsDescription>
      <DocsBody>{content}</DocsBody>
      <PageHelpfulness path={page.url} />
    </DocsPage>
  );
}

export async function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: PageProps<'/[[...slug]]'>): Promise<Metadata> {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  return {
    title: page.data.title,
    description: page.data.description,
    alternates: { canonical: page.url },
  };
}
