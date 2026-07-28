import { SupportedLanguage } from '@/lib/monaco';

export function getRepositoryFileLanguage(path: string): SupportedLanguage {
  const name = path.split('/').at(-1)?.toLowerCase() ?? '';

  if (name === 'dockerfile' || name.startsWith('dockerfile.')) return 'dockerfile';
  if (name === '.env' || name.startsWith('.env.')) return 'ini';
  if (name.endsWith('.yaml') || name.endsWith('.yml')) return 'yaml';
  if (name.endsWith('.json')) return 'json';
  if (name.endsWith('.xml') || name.endsWith('.csproj')) return 'xml';
  if (name.endsWith('.cs')) return 'csharp';
  if (name.endsWith('.tsx') || name.endsWith('.ts')) return 'typescript';
  if (name.endsWith('.jsx') || name.endsWith('.js')) return 'javascript';
  if (name.endsWith('.sh') || name.endsWith('.bash')) return 'shell';
  if (name.endsWith('.ps1')) return 'powershell';
  if (name.endsWith('.md')) return 'markdown';
  return 'plaintext';
}
