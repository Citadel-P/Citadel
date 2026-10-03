const Loader = ({ label = 'Loading…' }: { label?: string }) => {
  return (
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className="flex items-center justify-center gap-2.5 p-4 text-xs text-muted-foreground">
      <span
        aria-hidden="true"
        className="size-4 shrink-0 rounded-full border-[1.5px] border-muted-foreground/20 border-t-muted-foreground motion-safe:animate-spin"
      />
      <span>{label}</span>
    </div>
  );
};

export default Loader;
