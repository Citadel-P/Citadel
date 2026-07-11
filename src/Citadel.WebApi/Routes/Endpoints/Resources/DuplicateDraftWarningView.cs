using Application.Features;

namespace WebApi.Routes.Endpoints.Resources;

public sealed record DuplicateDraftWarningView(
    string Code,
    string Message,
    string? FieldPath = null)
{
    internal static DuplicateDraftWarningView Map(DuplicateDraftWarning warning) => new(
        warning.Code,
        warning.Message,
        warning.FieldPath);
}
