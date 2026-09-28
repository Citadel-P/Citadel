import { getValidationErrors } from '@/hooks/useGetValidationErrors';
import { toast } from 'sonner';

export function requestErrorStatus(error: unknown): number | undefined {
  const response = error as { status?: number; error?: { status?: number } } | null;
  return response?.status ?? response?.error?.status;
}

export function canShowCachedResource(error: unknown): boolean {
  return ![401, 403, 404].includes(requestErrorStatus(error) ?? 0);
}

export function requestErrorMessage(error: unknown): string {
  const status = requestErrorStatus(error);
  if (status === 401) return 'Your session has expired. Sign in again to continue.';
  if (status === 403) return 'You do not have permission to access this resource.';
  if (status === 404) return 'This resource could not be found. It may have been removed.';
  const problem = (error as { error?: { detail?: string; title?: string; errors?: Record<string, string[]> } } | null)
    ?.error;
  if (status === 400 && problem?.errors) return getValidationErrors(problem) || 'Check the form for validation errors.';
  if (problem?.detail) return problem.detail;
  if (problem?.title) return problem.title;
  return 'The request could not be completed. Check your connection and try again.';
}

// A mutation can reach both the cache subscriber and a confirmation handler.
// Report the same rejection once without swallowing it or closing the dialog.
const reported = new WeakSet<object>();
export function notifyRequestError(error: unknown) {
  if (typeof error === 'object' && error !== null) {
    if (reported.has(error)) return;
    reported.add(error);
    setTimeout(() => reported.delete(error), 0);
  }
  toast.error('Request failed', { description: requestErrorMessage(error) });
}
