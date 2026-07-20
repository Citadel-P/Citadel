import { useCallback } from 'react';
import { useLocation, useNavigate, useSearchParams } from 'react-router';

export function useBuildRunQuery() {
  const [searchParams] = useSearchParams();
  const location = useLocation();
  const navigate = useNavigate();
  const runId = searchParams.get('runId') ?? undefined;

  const clearRunId = useCallback((options?: { hash?: string | null }) => {
    if (!searchParams.has('runId') && options?.hash === undefined) return;

    const next = new URLSearchParams(searchParams);
    next.delete('runId');
    const search = next.toString();
    const hash = options?.hash === null ? '' : (options?.hash ?? location.hash);

    navigate(
      {
        pathname: location.pathname,
        search: search ? `?${search}` : '',
        hash,
      },
      { replace: true },
    );
  }, [location.hash, location.pathname, navigate, searchParams]);

  return { runId, hash: location.hash, clearRunId };
}
