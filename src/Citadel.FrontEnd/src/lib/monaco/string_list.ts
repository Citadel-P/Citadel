import { Monaco } from '@monaco-editor/react';

const string_list_conf = {
  comments: {
    lineComment: '#',
  },
  autoClosingPairs: [
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  surroundingPairs: [
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
};

const string_list_language = {
  defaultToken: '',
  tokenPostfix: '.string_list',

  tokenizer: {
    root: [
      // Comments
      [/#.*$/, 'comment'],

      // Comma as a delimiter
      [/,/, 'comment'],
      [/\*/, 'keyword'],
      [/\?/, 'keyword'],

      // Special syntax: text surrounded by \
      [/\\/, { token: 'keyword', next: '@regex' }],

      // Main strings separated by spaces or newlines
      [/[^\*\?,#\\\s]+/, ''],

      // Whitespace
      [/[ \t\r\n]+/, ''],
    ],
    regex: [
      // Regex tokens
      [/\[[^\]]*\]/, ''],
      [/[*+?\.]+/, 'keyword'],
      [/\\./, 'string.regexp constant.character.escape'],
      [/[^\\]/, 'string'],
      [/\\/, { token: 'keyword', next: '@pop' }],
    ],
  },
};

export const registerStringList = (monaco: Monaco) => {
  if (monaco.languages.getLanguages().some((l) => l.id === 'string_list')) {
    return;
  }

  monaco.languages.register({ id: 'string_list' });
  monaco.languages.setLanguageConfiguration('string_list', string_list_conf as any);
  monaco.languages.setMonarchTokensProvider('string_list', string_list_language as any);
};
