import { source } from './source';
import { legacyDocRedirects } from './doc-redirects.mjs';
import movedPages from '../../redirects.json';

export const legacyRoutes = legacyDocRedirects(
  source.getPages().map((page) => page.url),
  movedPages,
);
