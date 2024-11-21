import { ProblemDetails, ValidationProblemDetails } from '@/api/_generated';

export const useGetValidationErrors = (error: Error | null) => {
  if (error) {
    const validationProblem = (error as any).error as ValidationProblemDetails;
    if (validationProblem && validationProblem.status === 400) {
      if (validationProblem.errors) {
        return Object.values(validationProblem.errors).join(', ');
      }
    }
    const problem = (error as any).error as ProblemDetails;
    if (problem && problem.status && problem.status > 400 && problem.status <= 499) {
      return problem.detail;
    }
  }
};
