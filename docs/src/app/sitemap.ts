import { source } from '@/lib/source';
import { getSiteUrl } from '@/lib/shared';
import type { MetadataRoute } from 'next';

export const dynamic = 'force-static';

export default function sitemap(): MetadataRoute.Sitemap {
  const siteUrl = getSiteUrl();
  return [
    ...source.getPages().map((page) => ({
      url: `${siteUrl}${page.url}`,
      changeFrequency: 'weekly' as const,
      priority: page.slugs.length === 0 ? 1 : 0.7,
    })),
  ];
}
