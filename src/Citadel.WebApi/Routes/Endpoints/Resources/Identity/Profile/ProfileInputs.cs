using System.Reflection;
using System.Text.Json;
using Application.Features.Identity.Profile.Commands;
using Domain;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Identity.Profile;

public sealed record UpdateCurrentProfileInput(string DisplayName)
{
    internal UpdateCurrentProfile ToCommand() => new(DisplayName);
}

public sealed record PatchUserPreferencesInput(
    string? TimeZone,
    UserDateTimeFormat? DateTimeFormat,
    UserTheme? Theme);

public sealed class PatchUserPreferencesInputPatchDocument : JsonMergePatchDocument<PatchUserPreferencesInput>
{
    public static async ValueTask<PatchUserPreferencesInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new PatchUserPreferencesInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}

public sealed record ChangeCurrentPasswordInput(string CurrentPassword, string NewPassword)
{
    internal ChangeCurrentPassword ToCommand() => new(CurrentPassword, NewPassword);
}
