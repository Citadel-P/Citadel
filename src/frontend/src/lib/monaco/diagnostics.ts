// Monaco marker values, kept separate so validation does not load the editor runtime.
export const DiagnosticSeverity = { Hint: 1, Info: 2, Warning: 4, Error: 8 } as const;
export type DiagnosticSeverity = (typeof DiagnosticSeverity)[keyof typeof DiagnosticSeverity];
