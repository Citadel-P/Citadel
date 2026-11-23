import type { ReactNode } from 'react';
import { Wrench } from 'lucide-react';
import { Link } from 'react-router';
import AccessDenied from '@/assets/access-denied.svg?react';
import Ghost from '@/assets/error-ghost.svg?react';
import ServerError from '@/assets/500.svg?react';
import { Button } from '@/components/ui/button';

export function ErrorPanel({
  title,
  message,
  illustration,
  children,
}: {
  title: string;
  message: string;
  illustration: ReactNode;
  children: ReactNode;
}) {
  return (
    <div role="alert" className="flex justify-center px-4 py-4 sm:py-28 sm:px-6">
      <div className="flex w-full max-w-lg flex-col items-center gap-2 rounded-lg bg-background p-8 text-center shadow-xs">
        <h1 className="text-4xl font-bold text-foreground">{title}</h1>
        <p className="text-base text-muted-foreground">{message}</p>
        {illustration}
        <div className="flex flex-wrap justify-center gap-2">{children}</div>
      </div>
    </div>
  );
}

const errors = {
  403: { title: 'Access denied', message: 'You do not have permission to access this resource.' },
  404: { title: 'Oops!', message: 'We couldn’t find the page you were looking for.' },
  500: { title: 'Couldn’t load this page', message: 'Citadel encountered a server error while loading this page.' },
  503: {
    title: 'Update maintenance',
    message: 'Citadel is preparing an update. Try again after maintenance finishes.',
  },
} as const;

type Status = keyof typeof errors;

function ErrorIllustration({ status }: { status: Status }) {
  if (status === 500) return <ServerError role="img" aria-label="Error 500" className="my-3 h-auto w-full" />;
  return (
    <svg viewBox="0 0 975 415" className="my-3 w-full" role="img" aria-label={`Error ${status}`}>
      <text
        x="487.5"
        y="398"
        textAnchor="middle"
        fontSize="520"
        fontWeight="600"
        textLength="850"
        lengthAdjust="spacingAndGlyphs"
        fill="currentColor"
        opacity=".15">
        {status}
      </text>
      {status === 503 ? (
        <Wrench x="348" y="68" size={280} strokeWidth={1.25} />
      ) : status === 403 ? (
        <AccessDenied width="975" height="415" />
      ) : (
        <Ghost width="975" height="415" />
      )}
    </svg>
  );
}

export function PageError({
  status,
  message = errors[status].message,
  children,
}: {
  status: Status;
  message?: string;
  children?: ReactNode;
}) {
  return (
    <ErrorPanel title={errors[status].title} message={message} illustration={<ErrorIllustration status={status} />}>
      <Button asChild>
        <Link to="/">Go to homepage</Link>
      </Button>
      {children}
    </ErrorPanel>
  );
}
