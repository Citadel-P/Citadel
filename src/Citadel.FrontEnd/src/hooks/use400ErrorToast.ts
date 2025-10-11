import { HttpValidationProblemDetails, ProblemDetails } from '@/api/generated/api.types';
import { useEffect } from 'react';
import { toast } from 'sonner';

/**
 * A React hook to display a toast notification for HTTP 400 Bad Request errors.
 * This hook extends the global error handling provided by `useHTTPErrorHandler`.
 * It specifically customizes the handling of HTTP 400 errors, allowing for
 * a more granular display of validation messages from the API.
 *
 * @param error The error object, which may contain a `ProblemDetails` payload.
 * @param errorMessage Optional custom title for the toast notification. If not provided, a default title is generated.
 * @param on400ErrorHandled Optional callback function that is executed after the HTTP 400 error has been handled.
 */
export const use400ErrorToast = (error: Error | null, errorMessage?: string, on400ErrorHandled?: () => void) => {
  useEffect(() => {
    if (!error) return;
    const problem = (error as any)?.error as ProblemDetails;
    if (!problem) return;
    if (problem.status === 400) {
      const validationError = problem as HttpValidationProblemDetails;
      if (!validationError?.errors) return;

      // Display the first validation error message in the toast.
      const description = Object.values(validationError.errors)[0]?.[0];
      const title = errorMessage ?? `400: ${problem.title}`;
      toast.error(title, {
        description: description,
      });
      if (on400ErrorHandled) {
        on400ErrorHandled();
      }
    }
  }, [error, errorMessage, on400ErrorHandled]);
};
