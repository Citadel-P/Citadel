import { InfoIcon, Check, TriangleAlert, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ReactNode } from 'react';

interface IProps {
  type: 'success' | 'info' | 'warning' | 'error';
  hasTitle?: boolean | undefined;
  children?: string | ReactNode;
}

export const AlertMessage = ({ type, hasTitle, children }: IProps) => {
  return (
    <>
      {type === 'info' && <InfoMessage hasTitle={hasTitle}>{children}</InfoMessage>}
      {type === 'success' && <SuccessMessage hasTitle={hasTitle}>{children}</SuccessMessage>}
      {type === 'warning' && <WarningMessage hasTitle={hasTitle}>{children}</WarningMessage>}
      {type === 'error' && <ErrorMessage hasTitle={hasTitle}>{children}</ErrorMessage>}
    </>
  );
};

const SuccessMessage = ({ children, hasTitle = false }: Partial<IProps>) => {
  return (
    <Alert className="border-0 mt-2 mb-4 bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-100">
      <Check className="h-4 w-4 " />
      {hasTitle ? (
        <>
          <AlertTitle>Success!</AlertTitle>
          <AlertDescription>{children}</AlertDescription>
        </>
      ) : (
        <AlertTitle>{children}</AlertTitle>
      )}
    </Alert>
  );
};

const InfoMessage = ({ children, hasTitle = false }: Partial<IProps>) => {
  return (
    <Alert className="border-0 mt-2 mb-4 text-blue-800 bg-blue-100 dark:bg-blue-950 dark:text-blue-100">
      <InfoIcon className="h-4 w-4 " />
      {hasTitle ? (
        <>
          <AlertTitle>Info!</AlertTitle>
          <AlertDescription>{children}</AlertDescription>
        </>
      ) : (
        <AlertTitle>{children}</AlertTitle>
      )}
    </Alert>
  );
};

const WarningMessage = ({ children, hasTitle = false }: Partial<IProps>) => {
  return (
    <Alert className="border-0 mt-2 mb-4 bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-100">
      <AlertCircle className="h-4 w-4 " />
      {hasTitle ? (
        <>
          <AlertTitle>Heads up!</AlertTitle>
          <AlertDescription>{children}</AlertDescription>
        </>
      ) : (
        <AlertTitle>{children}</AlertTitle>
      )}
    </Alert>
  );
};

const ErrorMessage = ({ children, hasTitle = false }: Partial<IProps>) => {
  return (
    <Alert className="border-danger mt-2 mb-4 bg-red-50 text-red-600 dark:bg-red-950 dark:text-red-100">
      <TriangleAlert className="h-4 w-4 " />
      {hasTitle ? (
        <>
          <AlertTitle>Error!</AlertTitle>
          <AlertDescription>{children}</AlertDescription>
        </>
      ) : (
        <AlertTitle>{children}</AlertTitle>
      )}
    </Alert>
  );
};
