import Link from 'next/link';

export default function NotFound() {
  return (
    <main className="mx-auto flex min-h-[70vh] max-w-2xl flex-col items-center justify-center px-6 text-center">
      <p className="text-sm font-semibold text-fd-primary">404</p>
      <h1 className="mt-3 text-3xl font-semibold">Documentation page not found</h1>
      <p className="mt-3 text-fd-muted-foreground">
        The page may have moved or may not belong to this Citadel release.
      </p>
      <Link className="mt-7 rounded-md bg-fd-primary px-4 py-2 text-fd-primary-foreground" href="/docs">
        Browse documentation
      </Link>
    </main>
  );
}
