namespace Application.Features;

public sealed record DuplicateDraftWarning(
    string Code,
    string Message,
    string? FieldPath = null);
