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
  useEffect(() => client.start(), [client]);
  useEffect(() => client.trackPage(pathname), [client, pathname, state.choice, state.enabled]);
  const visible = state.enabled && !state.choice && client.isPage(pathname);
  return <Context.Provider value={client}>
    {children}
    {visible && <section hidden={state.editing} className="docs-analytics-notice" aria-label="Optional analytics">
      <strong>Help improve Citadel</strong>
      <p>
        Analytics help us understand what’s useful. Reject non-essentials keeps basic analytics.{' '}
        <span className="docs-analytics-description-links">
          <Link href="/privacy">Privacy details</Link>{' · '}
          <button className="docs-analytics-inline-preferences" type="button" aria-label="Manage preferences"
            onClick={() => client.edit(true)}>Preferences</button>
        </span>
      </p>
      <div className="docs-analytics-actions">
        <button type="button" onClick={() => client.choose('basic')}>Reject non-essentials</button>
        <button type="button" className="docs-analytics-accept" onClick={() => client.choose('accepted')}>Accept all</button>
      </div>
    </section>}
    {state.enabled && state.editing && <PreferencesDialog client={client} choice={state.choice} />}
  </Context.Provider>;
}

function PreferencesDialog({ client, choice }: { client: Client; choice: Parameters<Client['choose']>[0] | null }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [selected, setSelected] = useState(choice ?? 'basic');
  useEffect(() => {
    const trigger = document.activeElement;
    const element = dialog.current;
    const overflow = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    element?.showModal();
    element?.focus({ preventScroll: true });
    return () => {
      element?.close();
      document.body.style.overflow = overflow;
      if (trigger instanceof HTMLElement && trigger.isConnected) trigger.focus({ preventScroll: true });
    };
  }, []);
  return <dialog ref={dialog} tabIndex={-1} className="docs-analytics-dialog"
    aria-labelledby="docs-analytics-title" aria-describedby="docs-analytics-description"
    onCancel={() => client.edit(false)}
    onKeyDown={(event) => {
      if (event.key !== 'Tab') return;
      const controls = event.currentTarget.querySelectorAll<HTMLElement>('button, a[href], input');
      const first = controls[0];
      const last = controls[controls.length - 1];
      if (event.shiftKey && (document.activeElement === first || document.activeElement === event.currentTarget)) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    }}>
    <div className="docs-analytics-heading">
      <h2 id="docs-analytics-title">Analytics preferences</h2>
      <button className="docs-analytics-close" type="button" aria-label="Close" onClick={() => client.edit(false)}>
        <span aria-hidden="true">×</span>
      </button>
    </div>
    <p id="docs-analytics-description">
      Choose how you help improve Citadel.{' '}
      <Link href="/privacy" onClick={() => client.edit(false)}>Privacy details</Link>
    </p>
    <form onSubmit={(event) => { event.preventDefault(); client.choose(selected); }}>
      <fieldset className="docs-analytics-options">
        <legend className="sr-only">Analytics level</legend>
        {([
          ['basic', 'Basic', 'Public page visits and selected interactions.'],
          ['accepted', 'Full', 'Broader usage measurement with additional event details.'],
          ['declined', 'Off', 'No analytics collected.'],
        ] as const).map(([value, title, description]) => <label key={value} className="docs-analytics-option">
          <span><strong>{title}</strong><span>{description}</span></span>
          <input type="radio" name="analytics-level" value={value} aria-label={title}
            checked={selected === value} onChange={() => setSelected(value)} />
        </label>)}
      </fieldset>
      <div className="docs-analytics-actions">
        <button type="submit" className="docs-analytics-accept">Save preferences</button>
      </div>
    </form>
  </dialog>;
}

export function PageHelpfulness({ path }: { path: string }) {
  const client = useDocsAnalytics();
  return client ? <Helpfulness client={client} path={path} /> : null;
}

function Helpfulness({ client, path }: { client: Client; path: string }) {
  const state = useSyncExternalStore(client.subscribe, client.snapshot, client.serverSnapshot);
  if (!state.collecting || !client.canCollectPage(path)) return null;
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
