'use client';

import { useEffect } from 'react';
import { redirectDestination } from '@/lib/doc-redirects.mjs';

export function DocRedirect({ rule, basePath }: {
  rule: { to: string; anchors?: Record<string, string> };
  basePath: string;
}) {
  const destination = redirectDestination(rule, '', '', basePath);

  useEffect(() => {
    window.location.replace(
      redirectDestination(rule, window.location.hash, window.location.search, basePath),
    );
  }, [rule, basePath]);

  return <p>This page has moved. <a href={destination}>Open its new location</a>.</p>;
}
