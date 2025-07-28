import { HttpValidationProblemDetails, ProblemDetails } from '@/api/_generated';
import { useEffect } from 'react';
import { toast } from 'sonner';

// This hook extends the global error handling provided by `useHTTPErrorHandler`.// It specifically customizes the handling of HTTP 400 (Bad Request) errors,
// allowing for more granular display of validation messages.
export const use400ErrorToast = (error: Error | null, errorMessage?: string, on400ErrorHandled?: () => void) => {
  useEffect(() => {
    if (!error) return;
    const problem = (error as any)?.error as ProblemDetails;
    if (!problem) return;
    if (problem.status === 400) {
      const validationError = problem as HttpValidationProblemDetails;
      if (!validationError) return;

      let description = Object.values(validationError.errors)[0];
      const title = errorMessage ?? '400: ' + problem.title;
      toast.error(title, {
        description: description,
      });
      if (on400ErrorHandled) {
        on400ErrorHandled();
      }
    }
  }, [error]);
};
