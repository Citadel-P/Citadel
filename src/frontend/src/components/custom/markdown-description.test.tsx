import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MarkdownDescription, MarkdownDescriptionEditor } from './markdown-description';

describe('Markdown descriptions', () => {
  it('renders supported formatting and preserves ordinary multiline text', () => {
    const { container } = render(
      <MarkdownDescription>
        {
          'api_service\nSecond line\n\n**Important** and *optional* with `docker ps`\n\n- One\n- Two\n\n1. First\n2. Second\n\n> Note'
        }
      </MarkdownDescription>,
    );
    expect(container.querySelector('p')?.textContent).toBe('api_service\nSecond line');
    expect(container.querySelector('strong')).toHaveTextContent('Important');
    expect(container.querySelector('em')).toHaveTextContent('optional');
    expect(container.querySelector('code')).toHaveTextContent('docker ps');
    expect(screen.getAllByRole('list')).toHaveLength(2);
    expect(container.querySelector('blockquote')).toHaveTextContent('Note');
  });

  it('renders safe links without enabling HTML, images or executable URLs', () => {
    const { container } = render(
      <MarkdownDescription>
        {
          '[Runbook](https://example.com/runbook)\n\n[Unsafe](javascript:alert%281%29)\n\n[Data](data:text/html,hello)\n\n<img src="x" onerror="alert(1)">\n\n![Tracking](https://example.com/pixel.png)'
        }
      </MarkdownDescription>,
    );
    const link = screen.getByRole('link', { name: 'Runbook' });
    expect(link).toHaveAttribute('href', 'https://example.com/runbook');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    expect(screen.getAllByRole('link')).toHaveLength(1);
    expect(screen.getByText('Unsafe')).toBeVisible();
    expect(screen.getByText('Data')).toBeVisible();
    expect(container.querySelector('img, script')).toBeNull();
  });

  it('opens keyboard-accessible help without submitting or changing the description', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn((event) => event.preventDefault());
    const onChange = vi.fn();
    render(
      <form onSubmit={onSubmit}>
        <MarkdownDescriptionEditor aria-label="Description" value="**Draft**" onChange={onChange} />
      </form>,
    );
    const help = screen.getByRole('button', { name: 'Markdown formatting help' });
    help.focus();
    await user.keyboard('{Enter}');
    const dialog = screen.getByRole('dialog', { name: 'Formatting descriptions' });
    expect(within(dialog).getByText('**Important**')).toBeVisible();
    expect(within(dialog).getByText('[Runbook](https://example.com)')).toBeVisible();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(help).toHaveFocus();
    expect(screen.getByRole('textbox')).toHaveValue('**Draft**');
    expect(onSubmit).not.toHaveBeenCalled();
    expect(onChange).not.toHaveBeenCalled();
  });
});
