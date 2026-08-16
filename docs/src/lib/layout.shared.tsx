import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import Image from 'next/image';
import { appName, getApiDocsUrl, gitConfig } from './shared';

export function baseOptions(): BaseLayoutProps {
  const apiDocsUrl = getApiDocsUrl();
  const links: NonNullable<BaseLayoutProps['links']> = [
    {
      text: 'Report an issue',
      url: `https://github.com/${gitConfig.user}/${gitConfig.repo}/issues/new`,
      external: true,
    },
  ];
  if (apiDocsUrl) {
    links.unshift({ text: 'API Preview', url: apiDocsUrl, external: true });
  }

  return {
    nav: {
      title: (
        <span className="flex items-center gap-2 font-semibold">
          <Image src="/logo.svg" alt="" width={24} height={24} aria-hidden="true" />
          {appName}
        </span>
      ),
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
    links,
  };
}
