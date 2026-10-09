import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import Image from 'next/image';
import { appName, docsAssetUrl, gitConfig } from './shared';

export function baseOptions(
  links: NonNullable<BaseLayoutProps['links']> = [
    { text: 'API Preview', url: '/docs/reference/api' },
    {
      text: 'Report an issue',
      url: `https://github.com/${gitConfig.user}/${gitConfig.repo}/issues/new`,
      external: true,
    },
  ],
): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="flex items-center gap-2 font-semibold">
          <Image src={docsAssetUrl('/logo.svg')} alt="" width={24} height={24} aria-hidden="true" />
          {appName}
        </span>
      ),
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
    links: [...links, { text: 'Licensing', url: '/docs/overview/licensing' }],
  };
}
