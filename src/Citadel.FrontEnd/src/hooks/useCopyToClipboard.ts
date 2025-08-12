import { useCallback, useState, useEffect } from 'react';

type CopiedValue = string | null;

type CopyFn = (text: string) => Promise<boolean>;

/**
 * A React hook that provides a function to copy text to the clipboard.
 *
 * @param {number} [clearTimer] - The time in milliseconds after which the copied text state is cleared. If not provided, the state will not be cleared automatically.
 * @returns {[CopiedValue, CopyFn]} A tuple containing the copied text and the copy function.
 */
export function useCopyToClipboard(clearTimer?: number): [CopiedValue, CopyFn] {
  const [copiedText, setCopiedText] = useState<CopiedValue>(null);

  const copy: CopyFn = useCallback(async (text) => {
    if (!navigator?.clipboard) {
      console.warn('Clipboard not supported');
      return false;
    }

    // Attempt to write the text to the clipboard.
    try {
      await navigator.clipboard.writeText(text);
      setCopiedText(text);
      return true;
    } catch (error) {
      console.warn('Copy failed', error);
      setCopiedText(null);
      return false;
    }
  }, []);

  useEffect(() => {
    if (copiedText && clearTimer) {
      const timeoutId = setTimeout(() => {
        setCopiedText(null);
      }, clearTimer);

      return () => {
        clearTimeout(timeoutId);
      };
    }
  }, [copiedText, clearTimer]);

  return [copiedText, copy];
}
