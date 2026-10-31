'use client';
import {
  SearchDialog,
  SearchDialogClose,
  SearchDialogContent,
  SearchDialogHeader,
  SearchDialogIcon,
  SearchDialogInput,
  SearchDialogList,
  SearchDialogOverlay,
  type SharedProps,
} from 'fumadocs-ui/components/dialog/search';
import { useDocsSearch } from 'fumadocs-core/search/client';
import { staticClient } from 'fumadocs-core/search/client/orama-static';
import { useI18n } from 'fumadocs-ui/contexts/i18n';
import { docsAssetUrl } from '@/lib/shared';
import { useEffect, useRef } from 'react';
import { useDocsAnalytics } from '@/components/analytics/analytics';

export default function DefaultSearchDialog(props: SharedProps) {
  const analytics = useDocsAnalytics();
  const completed = useRef(false);
  useEffect(() => { if (props.open) completed.current = false; }, [props.open]);
  const { locale } = useI18n(); // (optional) for i18n
  const { search, setSearch, query } = useDocsSearch({
    client: staticClient({
      locale,
      from: docsAssetUrl('/api/search'),
    }),
  });

  function completeSearch() {
    // Count the final settled query once when this interaction closes, not keystrokes.
    // Search text stays inside the search component and never reaches analytics.
    if (completed.current || !search.trim() || query.isLoading || query.error || query.data === 'empty') return;
    completed.current = true;
    analytics?.searchCompleted(query.data.length);
  }

  return (
    <SearchDialog search={search} isLoading={query.isLoading} {...props}
      onSearchChange={(value) => { completed.current = false; setSearch(value); }}
      onOpenChange={(open) => {
        if (open) completed.current = false;
        else completeSearch();
        props.onOpenChange(open);
      }}
      onSelect={(item) => {
        if (search.trim() && !query.isLoading && !query.error && item.type !== 'action' && !item.external) {
          analytics?.searchResultOpened(item.url);
        }
      }}>
      <SearchDialogOverlay />
      <SearchDialogContent>
        <SearchDialogHeader>
          <SearchDialogIcon />
          <SearchDialogInput />
          <SearchDialogClose />
        </SearchDialogHeader>
        <SearchDialogList items={query.data !== 'empty' ? query.data : null} />
      </SearchDialogContent>
    </SearchDialog>
  );
}
