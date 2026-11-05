'use client';

import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import { usePathname } from 'next/navigation';
import { Check, Clipboard } from 'lucide-react';
import { CodeBlock, Pre, type CodeBlockProps } from 'fumadocs-ui/components/codeblock';
import { useDocsAnalytics } from './analytics/analytics';

const Step = createContext<string | null>(null);

export function InstallationStep({ name, children }: {
  name: 'download' | 'start' | 'start_tls'; children: ReactNode;
}) {
  return <Step.Provider value={name}>{children}</Step.Provider>;
}

// Fumadocs has no copy-success callback. Keep its renderer and replace only the
// action on explicitly marked installation examples so failed copies never count.
export function DocumentationCodeBlock(props: CodeBlockProps) {
  const step = useContext(Step);
  return step && props.allowCopy !== false && props.allowCopy !== 'false'
    ? <InstallationCodeBlock {...props} step={step} />
    : <CodeBlock {...props}><Pre>{props.children}</Pre></CodeBlock>;
}

function InstallationCodeBlock({ step, ...props }: CodeBlockProps & { step: string }) {
  const analytics = useDocsAnalytics();
  const pathname = usePathname();
  const figure = useRef<HTMLElement>(null);
  const timeout = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const [status, setStatus] = useState('Copy Text');
  useEffect(() => () => clearTimeout(timeout.current), []);

  async function copy() {
    const pre = figure.current?.querySelector('pre');
    if (!pre) return;
    const clone = pre.cloneNode(true) as HTMLElement;
    clone.querySelectorAll('.nd-copy-ignore').forEach((node) => node.replaceWith('\n'));
    try {
      await navigator.clipboard.writeText(clone.textContent ?? '');
      analytics?.installCommandCopied(pathname, step);
      setStatus('Copied Text');
    } catch {
      setStatus('Copy failed. Select the commands to copy them.');
    }
    clearTimeout(timeout.current);
    timeout.current = setTimeout(() => setStatus('Copy Text'), 1500);
  }

  return <CodeBlock {...props} ref={figure} allowCopy={false} Actions={({ className }) => (
    <div className={className}>
      <button type="button" onClick={() => void copy()} aria-label={status}
        className="inline-flex size-7 items-center justify-center rounded-md hover:bg-fd-accent hover:text-fd-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-fd-ring">
        {status === 'Copied Text' ? <Check size={14} aria-hidden /> : <Clipboard size={14} aria-hidden />}
        <span className="sr-only" role="status">{status}</span>
      </button>
    </div>
  )}><Pre>{props.children}</Pre></CodeBlock>;
}
