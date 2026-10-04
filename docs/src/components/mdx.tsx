import defaultMdxComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import type { ImgHTMLAttributes } from 'react';
import { docsAssetUrl } from '@/lib/shared';

function DocumentationImage(props: ImgHTMLAttributes<HTMLImageElement>) {
  const Image = defaultMdxComponents.img;
  const src = typeof props.src === 'string' && props.src.startsWith('/')
    ? docsAssetUrl(props.src)
    : props.src;
  return <Image {...props} src={src} alt={props.alt ?? ''} />;
}

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    img: DocumentationImage,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
