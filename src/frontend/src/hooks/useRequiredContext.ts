import { Context, useContext } from 'react';

/**
 * A React hook that provides a way to consume a context and ensures that the context is not null or undefined.
 * It throws an error if the context is used outside of its provider, which helps to catch bugs early.
 *
 * @template TContext The type of the context value.
 * @param {Context<TContext>} context The React context to consume.
 * @returns {NonNullable<TContext>} The context value, guaranteed to be non-nullable.
 * @throws {Error} If the context is used outside of its provider.
 */
export const useRequiredContext = <TContext>(context: Context<TContext>): NonNullable<TContext> => {
  const usedContext = useContext(context);
  if (usedContext === undefined) {
    throw new Error(`'${context.displayName || 'Unknown'}' must be used within a Provider`);
  }
  return usedContext as NonNullable<TContext>;
};
