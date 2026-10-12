import { readdirSync, readFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import ts from 'typescript';
import { describe, expect, it } from 'vitest';

const root = resolve(process.cwd(), 'src');

function sources(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) return entry.name === 'test' ? [] : sources(path);
    return /\.(ts|tsx)$/.test(path) && !/\.(test|spec|d)\.tsx?$/.test(path) ? [path] : [];
  });
}

function imports(file: string, source: string): string[] {
  const dependencies: string[] = [];
  const visit = (node: ts.Node) => {
    const specifier =
      ts.isImportDeclaration(node) || ts.isExportDeclaration(node)
        ? node.moduleSpecifier
        : ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword
          ? node.arguments[0]
          : undefined;
    if (specifier && ts.isStringLiteral(specifier)) {
      const name = specifier.text;
      if (name.startsWith('@/') || name.startsWith('.')) {
        const target = name.startsWith('@/') ? resolve(root, name.slice(2)) : resolve(dirname(file), name);
        dependencies.push(relative(root, target).replaceAll('\\', '/'));
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true));
  return dependencies;
}

function violation(owner: string, dependency: string): boolean {
  if (/^(main|router)(\.|$)/.test(dependency)) return true;
  if (owner.startsWith('api/')) return /^(features|pages|layout|components)\//.test(dependency);
  if (owner.startsWith('components/ui/')) return /^(features|pages|layout|api|components\/custom)\//.test(dependency);
  return false;
}

describe('frontend dependency boundaries', () => {
  it('keeps the API and UI primitives independent of resource screens and routing', () => {
    const violations: string[] = [];
    for (const file of sources(root)) {
      const owner = relative(root, file).replaceAll('\\', '/');
      if (owner === 'main.tsx') continue; // Application composition owns the router.
      for (const dependency of imports(file, readFileSync(file, 'utf8'))) {
        if (violation(owner, dependency)) violations.push(`${owner} -> ${dependency}`);
      }
    }
    expect(violations).toEqual([]);
  });

  it('checks aliases, relative paths, re-exports and lazy imports', () => {
    const dependencies = imports(
      resolve(root, 'api/example.ts'),
      `
      import type { Model } from '@/features/example';
      export { Screen } from '../pages/example';
      const load = () => import('../router');
    `,
    );
    expect(dependencies).toEqual(['features/example', 'pages/example', 'router']);
    expect(dependencies.every((dependency) => violation('api/example.ts', dependency))).toBe(true);
    // Shared page templates are deliberately consumed by features in Citadel.
    expect(violation('features/example.ts', 'pages/types')).toBe(false);
    expect(violation('components/ui/button.tsx', 'lib/utils')).toBe(false);
  });
});
