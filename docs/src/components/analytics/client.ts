// Public docs only. Keep consent and payload boundaries aligned with Citadel.Web.
export const collector = 'https://analytics.citadelplane.com/docs';
const productionOrigin = 'https://docs.citadelplane.com';
const storageKey = 'citadel.docs.analytics-consent.v1';
const lifetime = 180 * 24 * 60 * 60 * 1000;
type Choice = 'accepted' | 'declined';
type Consent = { choice: Choice; expires: number };
type Config = { websiteId: string; hostname: string };
type State = { enabled: boolean; choice: Choice | null; editing: boolean; collecting: boolean };
const initial: State = { enabled: false, choice: null, editing: false, collecting: false };

export function createAnalytics(routes: Record<string, string>, siteUrl: string) {
  let state = initial;
  let consent: Consent | null = null;
  let config: Config | null = null;
  let lastPage: string | null = null;
  let expiry: ReturnType<typeof setTimeout> | undefined;
  const feedback = new Set<string>();
  const listeners = new Set<() => void>();
  const pending = new Set<AbortController>();
  const site = new URL(siteUrl);
  const basePath = site.pathname.replace(/\/+$/, '');
  const normalize = (path: string) => path.replace(/\/+$/, '') || '/';

  function permitted() {
    return config !== null && consent?.choice === 'accepted' && consent.expires > Date.now() &&
      navigator.doNotTrack !== '1' &&
      (navigator as Navigator & { globalPrivacyControl?: boolean }).globalPrivacyControl !== true;
  }
  function update() {
    state = { ...state, choice: consent?.choice ?? null, collecting: permitted() };
    listeners.forEach((listener) => listener());
  }
  function cancel() {
    pending.forEach((controller) => controller.abort());
    pending.clear();
    lastPage = null;
  }
  function readConsent(): Consent | null {
    try {
      const value = JSON.parse(localStorage.getItem(storageKey) ?? 'null');
      if ((value?.choice === 'accepted' || value?.choice === 'declined') &&
          Number.isFinite(value.expires) && value.expires > Date.now() &&
          value.expires <= Date.now() + lifetime) return value;
    } catch { /* Storage is optional; default to no consent. */ }
    return null;
  }
  function checkExpiry() {
    clearTimeout(expiry);
    if (!consent) return;
    if (consent.expires <= Date.now()) {
      consent = null;
      cancel();
      update();
    } else {
      expiry = setTimeout(checkExpiry, Math.min(consent.expires - Date.now(), 2_147_483_647));
    }
  }
  function storageChanged(event: StorageEvent) {
    if (event.key !== storageKey && event.key !== null) return;
    consent = readConsent();
    cancel();
    checkExpiry();
    update();
  }

  function currentRoute() {
    const path = normalize(location.pathname);
    if (basePath && path !== basePath && !path.startsWith(`${basePath}/`)) return null;
    const route = normalize(path.slice(basePath.length));
    return Object.hasOwn(routes, route) ? route : null;
  }
  function send(route: string, event?: { name: string; data?: Record<string, string | number> }) {
    checkExpiry();
    if (!permitted() || !config || currentRoute() !== route) return false;
    const controller = new AbortController();
    pending.add(controller);
    const timeout = setTimeout(() => controller.abort(), 4000);
    // Construct payloads here; never spread caller-provided objects into telemetry.
    void fetch(`${collector}/api/send`, {
      method: 'POST', credentials: 'omit', referrerPolicy: 'no-referrer', cache: 'no-store',
      headers: { 'Content-Type': 'application/json' }, signal: controller.signal,
      body: JSON.stringify({ type: 'event', payload: {
        website: config.websiteId, hostname: config.hostname,
        url: `${basePath}${route}`, title: routes[route],
        ...(event ? { name: event.name, ...(event.data ? { data: event.data } : {}) } : {}),
      } }),
    }).catch(() => undefined).finally(() => {
      clearTimeout(timeout);
      pending.delete(controller);
    });
    return true;
  }

  return {
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    snapshot: () => state,
    serverSnapshot: () => initial,
    start() {
      // A production build served on a preview or localhost must still send nothing.
      if (site.origin !== productionOrigin || location.origin !== site.origin) return;
      let stopped = false;
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 4000);
      consent = readConsent();
      checkExpiry();
      window.addEventListener('storage', storageChanged);
      document.addEventListener('visibilitychange', checkExpiry);
      void fetch(`${collector}/config.json`, {
        credentials: 'omit', referrerPolicy: 'no-referrer', cache: 'no-store',
        signal: controller.signal,
      }).then(async (response) => {
        if (!response.ok || !response.headers.get('content-type')?.includes('application/json')) return;
        const value = await response.json();
        if (stopped || value?.enabled !== true || value.hostname !== site.hostname ||
            typeof value.websiteId !== 'string' ||
            !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value.websiteId)) return;
        config = { websiteId: value.websiteId, hostname: value.hostname };
        state = { ...state, enabled: true };
        update();
      }).catch(() => undefined).finally(() => clearTimeout(timeout));
      return () => {
        stopped = true;
        controller.abort();
        clearTimeout(timeout);
        clearTimeout(expiry);
        window.removeEventListener('storage', storageChanged);
        document.removeEventListener('visibilitychange', checkExpiry);
        cancel();
        config = null;
        state = initial;
      };
    },
    choose(choice: Choice) {
      consent = { choice, expires: Date.now() + lifetime };
      try { localStorage.setItem(storageKey, JSON.stringify(consent)); } catch { /* Tab-only choice. */ }
      state = { ...state, editing: false };
      cancel();
      checkExpiry();
      update();
    },
    edit(editing: boolean) {
      state = { ...state, editing };
      update();
    },
    isPage: (path: string) => Object.hasOwn(routes, normalize(path)),
    trackPage(path: string) {
      const route = normalize(path);
      if (!Object.hasOwn(routes, route)) { lastPage = null; return; }
      if (lastPage !== route && send(route)) lastPage = route;
    },
    searchCompleted(resultCount: number) {
      const route = currentRoute();
      if (!route || !Number.isSafeInteger(resultCount) || resultCount < 0) return;
      if (send(route, { name: 'search_used', data: { result_count: Math.min(resultCount, 1000) } }) && resultCount === 0) {
        send(route, { name: 'search_no_results' });
      }
    },
    searchResultOpened(url: string) {
      const route = currentRoute();
      if (!route) return;
      // Search URLs are app-relative; send only an exact public source-page match.
      try {
        const target = new URL(url, site.origin);
        const selected = normalize(target.pathname);
        if (target.origin !== site.origin || !Object.hasOwn(routes, selected)) return;
        send(route, { name: 'search_result_opened', data: { selected_page: `${basePath}${selected}` } });
      } catch { /* An invalid search result is never telemetry. */ }
    },
    hasFeedback: (path: string) => feedback.has(normalize(path)),
    ratePage(path: string, helpful: boolean) {
      const route = normalize(path);
      if (typeof helpful !== 'boolean' || feedback.has(route)) return false;
      if (!send(route, { name: helpful ? 'page_helpful' : 'page_unhelpful' })) return false;
      feedback.add(route);
      update();
      return true;
    },
  };
}
