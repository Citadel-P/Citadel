'use client';

import { createContext, useContext, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { usePathname } from 'next/navigation';
import Link from 'next/link';
import { createAnalytics } from './client';
import './analytics.css';

type Client = ReturnType<typeof createAnalytics>;
const Context = createContext<Client | null>(null);
export const useDocsAnalytics = () => useContext(Context);

export function AnalyticsPreferences() {
  const client = useContext(Context);
  return client ? <Preferences client={client} /> : null;
}

function Preferences({ client }: { client: Client }) {
  const state = useSyncExternalStore(client.subscribe, client.snapshot, client.serverSnapshot);
  if (!state.enabled) return null;
  return <button className="docs-analytics-preferences" type="button" onClick={() => client.edit(true)}>
    Analytics preferences
  </button>;
}

export function Analytics({ routes, siteUrl, children }: {
  routes: Record<string, string>; siteUrl: string; children: React.ReactNode;
}) {
  const [client] = useState(() => createAnalytics(routes, siteUrl));
  const state = useSyncExternalStore(client.subscribe, client.snapshot, client.serverSnapshot);
  const pathname = usePathname();
  const notice = useRef<HTMLElement>(null);
  useEffect(() => client.start(), [client]);
  useEffect(() => client.trackPage(pathname), [client, pathname, state.choice, state.enabled]);
  useEffect(() => {
    if (!state.editing) return;
    const trigger = document.activeElement;
    notice.current?.focus({ preventScroll: true });
    return () => {
      if (trigger instanceof HTMLElement && trigger.isConnected) trigger.focus({ preventScroll: true });
    };
  }, [state.editing]);
  const visible = state.enabled && (state.editing || (!state.choice && client.isPage(pathname)));
  return <Context.Provider value={client}>
    {children}
    {visible && <section ref={notice} tabIndex={-1} className="docs-analytics-notice" aria-label="Optional analytics">
      <div>
        <strong>Help improve the docs</strong>
        <p>Allow optional analytics for page views, search outcomes and page feedback? <Link href="/privacy">Privacy details</Link></p>
      </div>
      <div className="docs-analytics-actions">
        <button type="button" onClick={() => client.choose('declined')}>Decline</button>
        <button type="button" onClick={() => client.choose('accepted')}>Accept</button>
        {state.editing && state.choice && <button type="button" onClick={() => client.edit(false)}>Close</button>}
      </div>
    </section>}
  </Context.Provider>;
}

export function PageHelpfulness({ path }: { path: string }) {
  const client = useDocsAnalytics();
  return client ? <Helpfulness client={client} path={path} /> : null;
}

function Helpfulness({ client, path }: { client: Client; path: string }) {
  const state = useSyncExternalStore(client.subscribe, client.snapshot, client.serverSnapshot);
  if (!state.collecting) return null;
  const answered = client.hasFeedback(path);
  return <section className="docs-helpfulness" aria-label="Page feedback">
    <span>Was this page helpful?</span>
    <div className="docs-analytics-actions">
      <button type="button" disabled={answered} aria-label="Yes, this page was helpful"
        onClick={() => client.ratePage(path, true)}>Yes</button>
      <button type="button" disabled={answered} aria-label="No, this page was not helpful"
        onClick={() => client.ratePage(path, false)}>No</button>
    </div>
    <span role="status">{answered ? 'Thank you for your feedback.' : ''}</span>
  </section>;
}
