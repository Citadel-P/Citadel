import { source } from '@/lib/source';
import { DocumentationLayout } from '@/components/documentation-layout';
import { baseOptions } from '@/lib/layout.shared';

export default function Layout({ children }: LayoutProps<'/docs'>) {
  return (
    <DocumentationLayout tree={source.getPageTree()} {...baseOptions()}>
      {children}
    </DocumentationLayout>
  );
}
