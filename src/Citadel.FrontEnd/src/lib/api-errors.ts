import { ProblemDetails } from '@/api/generated/api.types';

export const getApiErrorDetail = (error: unknown, fallback: string): string => {
  const problem = (error as { error?: ProblemDetails } | null | undefined)?.error;
  if (problem?.detail?.trim()) return problem.detail;
  if (problem?.title?.trim()) return problem.title;
  if (error instanceof Error && error.message.trim()) return error.message;
  return fallback;
};
