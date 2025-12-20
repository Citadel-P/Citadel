export const ContentCard = ({ children, className }: { children: React.ReactNode; className?: string }) => (
  <div className={`rounded-sm border p-1 shadow-xs ${className || ''}`}>{children}</div>
);
