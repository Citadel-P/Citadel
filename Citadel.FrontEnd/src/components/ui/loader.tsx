const Loader = () => {
  return (
    <div className="flex items-center justify-center p-4">
      <div className="animate-pulse rounded-full bg-primary/20 dark:text-foreground px-3 py-1 text-center text-xs font-medium leading-none">
        loading...
      </div>
    </div>
  );
};

export default Loader;
