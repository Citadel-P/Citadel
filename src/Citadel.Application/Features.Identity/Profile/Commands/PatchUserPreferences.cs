using System.Text.Json;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Commands;

public sealed record PatchUserPreferences(JsonMergePatchDocument<PatchUserPreferencesModel> Patch)
    : ICommand<Result<UserPreferencesDetails>>
{
    internal sealed class Validator : PatchCommandValidator<PatchUserPreferences, PatchUserPreferencesModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: ProfileJsonContext.Default.PatchUserPreferencesModel,
                modelValidator: new PatchUserPreferencesModelValidator())
        {
            RuleFor(x => x)
                .Custom((command, context) =>
                {
                    if (command.Patch.Patch.ValueKind != JsonValueKind.Object)
                    {
                        context.AddFailure("Patch", "Patch must be a JSON object.");
                        return;
                    }

                    var found = false;
                    foreach (var property in command.Patch.Patch.EnumerateObject())
                    {
                        if (!IsPreferenceProperty(property.Name))
                            continue;

                        found = true;
                        if (property.Value.ValueKind == JsonValueKind.Null)
                            context.AddFailure(property.Name, $"{property.Name} cannot be null.");
                    }

                    if (!found)
                        context.AddFailure("Patch", "At least one preference field is required.");
                });
        }
    }

    private sealed class PatchUserPreferencesModelValidator : AbstractValidator<PatchUserPreferencesModel>
    {
        public PatchUserPreferencesModelValidator()
        {
            When(x => x.TimeZone is not null, () =>
            {
                RuleFor(x => x.TimeZone!)
                    .NotEmpty()
                    .Must(IsValidTimeZone)
                    .WithMessage("TimeZone must be a valid IANA timezone.");
            });
        }
    }

    private static bool IsPreferenceProperty(string name)
        => string.Equals(name, "timeZone", StringComparison.OrdinalIgnoreCase)
           || string.Equals(name, "dateTimeFormat", StringComparison.OrdinalIgnoreCase)
           || string.Equals(name, "theme", StringComparison.OrdinalIgnoreCase);

    private static bool IsValidTimeZone(string timeZone)
    {
        try
        {
            _ = TimeZoneInfo.FindSystemTimeZoneById(timeZone);
            return true;
        }
        catch (TimeZoneNotFoundException)
        {
            return false;
        }
        catch (InvalidTimeZoneException)
        {
            return false;
        }
    }
}

internal sealed class PatchUserPreferencesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : ICommandHandler<PatchUserPreferences, Result<UserPreferencesDetails>>
{
    public async ValueTask<Result<UserPreferencesDetails>> Handle(PatchUserPreferences command, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        var actorId = userContext.Current.ActorId;
        if (userId == Guid.Empty || actorId == Guid.Empty)
            return Result.Failure<UserPreferencesDetails>(new UnauthorizedError("Missing user context"));

        var provided = GetProvidedProperties(command.Patch.Patch);
        var current = await unitOfWork.UserPreferences.GetAsync(userId, cancellationToken);
        var currentModel = new PatchUserPreferencesModel(
            current?.TimeZone ?? "UTC",
            current?.DateTimeFormat ?? UserDateTimeFormat.System,
            current?.Theme ?? UserTheme.System);

        var patched = command.Patch.ApplyTo(currentModel, ProfileJsonContext.Default.PatchUserPreferencesModel);
        var timeZone = patched.TimeZone ?? currentModel.TimeZone!;
        if (!IsValidTimeZone(timeZone))
            return Result.Failure<UserPreferencesDetails>(new BadRequestError("TimeZone must be a valid IANA timezone."));

        var dateTimeFormat = patched.DateTimeFormat ?? currentModel.DateTimeFormat!.Value;
        var theme = patched.Theme ?? currentModel.Theme!.Value;

        var changes = BuildChanges(current, provided, timeZone, dateTimeFormat, theme);
        var shouldPersist = current is null || changes.Count > 0;
        if (shouldPersist)
        {
            var now = DateTime.UtcNow;
            var preferences = current ?? UserPreferences.Create(userId, timeZone, dateTimeFormat, theme, now);
            preferences.Update(timeZone, dateTimeFormat, theme, now);

            await unitOfWork.UserPreferences.UpsertAsync(preferences, cancellationToken);

            if (changes.Count > 0)
            {
                var profile = await unitOfWork.Users.GetCurrentProfileAsync(userId, cancellationToken);
                await unitOfWork.ActivityEventRepository.AddAsync(
                    new ActivityEvent(
                        platformId: null,
                        resourceId: userId,
                        actorId: actorId,
                        resourceName: profile?.DisplayName ?? "Current user",
                        eventType: ActivityEventType.UserPreferencesUpdated,
                        status: ActivityStatus.Success,
                        info: new UserPreferencesUpdated(changes)),
                    cancellationToken);
            }

            await unitOfWork.CommitAsync(cancellationToken);
            current = preferences;
        }

        return Result.Success(new UserPreferencesDetails(
            current?.TimeZone ?? timeZone,
            current?.DateTimeFormat ?? dateTimeFormat,
            current?.Theme ?? theme,
            true));
    }

    private static HashSet<string> GetProvidedProperties(JsonElement patch)
    {
        var provided = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var property in patch.EnumerateObject())
        {
            if (string.Equals(property.Name, "timeZone", StringComparison.OrdinalIgnoreCase))
                provided.Add("TimeZone");
            else if (string.Equals(property.Name, "dateTimeFormat", StringComparison.OrdinalIgnoreCase))
                provided.Add("DateTimeFormat");
            else if (string.Equals(property.Name, "theme", StringComparison.OrdinalIgnoreCase))
                provided.Add("Theme");
        }

        return provided;
    }

    private static List<ActivityChangedField> BuildChanges(
        UserPreferences? current,
        HashSet<string> provided,
        string timeZone,
        UserDateTimeFormat dateTimeFormat,
        UserTheme theme)
    {
        var changes = new List<ActivityChangedField>(3);
        if (provided.Contains("TimeZone") && !string.Equals(current?.TimeZone, timeZone, StringComparison.Ordinal))
            changes.Add(new ActivityChangedField("TimeZone", current?.TimeZone, timeZone));

        var oldDateTimeFormat = current?.DateTimeFormat ?? UserDateTimeFormat.System;
        if (provided.Contains("DateTimeFormat") && oldDateTimeFormat != dateTimeFormat)
            changes.Add(new ActivityChangedField("DateTimeFormat", oldDateTimeFormat.ToString(), dateTimeFormat.ToString()));

        var oldTheme = current?.Theme ?? UserTheme.System;
        if (provided.Contains("Theme") && oldTheme != theme)
            changes.Add(new ActivityChangedField("Theme", oldTheme.ToString(), theme.ToString()));

        return changes;
    }

    private static bool IsValidTimeZone(string timeZone)
    {
        try
        {
            _ = TimeZoneInfo.FindSystemTimeZoneById(timeZone);
            return true;
        }
        catch (TimeZoneNotFoundException)
        {
            return false;
        }
        catch (InvalidTimeZoneException)
        {
            return false;
        }
    }
}
