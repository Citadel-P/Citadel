import { Node, Project } from 'ts-morph';

/** Read the client we actually generated, including non-api namespaces. */
export function clientSignatures(source: string) {
  const project = new Project({ useInMemoryFileSystem: true });
  const file = project.createSourceFile('client.ts', source);
  const signatures = new Map<string, { namespace: string; params: string[]; requiredParams: string[] }>();
  for (const property of file.getClassOrThrow('Api').getProperties()) {
    const namespace = property.getName();
    const methods = property.getInitializer();
    if (!Node.isObjectLiteralExpression(methods)) continue;
    for (const method of methods.getProperties()) {
      if (!Node.isPropertyAssignment(method)) continue;
      const fn = method.getInitializer();
      if (!Node.isArrowFunction(fn)) continue;
      const key = method.getName();
      if (signatures.has(key)) throw new Error(`Ambiguous generated operation: ${key}`);
      signatures.set(key, {
        namespace,
        params: fn.getParameters().map((parameter) => parameter.getName()),
        requiredParams: fn
          .getParameters()
          .filter((parameter) => !parameter.isOptional())
          .map((parameter) => parameter.getName()),
      });
    }
  }
  return signatures;
}
