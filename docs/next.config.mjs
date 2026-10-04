import { createMDX } from 'fumadocs-mdx/next';
import { getDocsBasePath } from './site-config.mjs';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  output: 'export',
  basePath: getDocsBasePath(),
  reactStrictMode: true,
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
};

export default withMDX(config);
