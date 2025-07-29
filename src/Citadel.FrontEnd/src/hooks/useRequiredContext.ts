import { Context, useContext } from 'react';

export const useRequiredContext = <TContext>(context: Context<TContext>): NonNullable<TContext> => {
  const usedContext = useContext(context);
  if (usedContext === undefined) {
    throw new Error(`${context.displayName || ''}Context must be used within a ${context.displayName || ''} Provider`);
  }
  return usedContext as NonNullable<TContext>;
};
