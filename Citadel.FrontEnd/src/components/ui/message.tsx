import { InfoIcon, Check, TriangleAlert } from 'lucide-react';
import { ReactNode } from 'react';

interface IProps {
  type: 'success' | 'info' | 'warning' | 'error';
  children?: string | ReactNode;
}

export const Message = ({ type, children }: IProps) => {
  return (
    <>
      {type === 'info' && <InfoMessage>{children}</InfoMessage>}
      {type === 'success' && <SuccessMessage>{children}</SuccessMessage>}
      {type === 'warning' && <WarningMessage>{children}</WarningMessage>}
    </>
  );
};

const SuccessMessage = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="flex items-center p-4 mb-4 border-t-4 border-success-foreground/40 bg-success/80" role="alert">
      <Check className="text-success-foreground" />
      <div className="ml-3 text-sm font-medium text-success-foreground">{children}</div>
    </div>
  );
};

const InfoMessage = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="flex items-center p-4 mb-4 border-t-4 border-info-foreground/40 bg-info/80" role="alert">
      <InfoIcon className="text-info-foreground" />
      <div className="ml-3 text-sm font-medium text-info-foreground">{children}</div>
    </div>
  );
};

const WarningMessage = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className="flex items-center p-4 mb-4 border-t-4 border-warning-foreground/40 bg-warning/80" role="alert">
      <TriangleAlert className="text-warning-foreground" />
      <div className="ml-3 text-sm font-medium text-warning-foreground">{children}</div>
    </div>
  );
};
