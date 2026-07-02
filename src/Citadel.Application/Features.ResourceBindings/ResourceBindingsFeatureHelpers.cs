using Application.Features.ResourceBindings.Commands;
using Application.Features.ResourceBindings.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Features.ResourceBindings;

internal static class ResourceBindingsFeatureHelpers
{
    public static async Task<Result<ResourceBindingsResult>> CreateEntryAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid? resourceId,
        ResourceBindingInput input,
        CancellationToken cancellationToken)
    {
        if (scope == ResourceBindingScope.Global && resourceId is not null)
            return Result.Failure<ResourceBindingsResult>(new BadRequestError("Global resource bindings do not target a resource."));

        if (scope != ResourceBindingScope.Global && resourceId is null)
            return Result.Failure<ResourceBindingsResult>(new BadRequestError("Resource scoped bindings require a resource id."));

        var existing = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        if (existing.Any(entry => string.Equals(entry.Name, input.Name, StringComparison.Ordinal)))
            return Result.Failure<ResourceBindingsResult>(new ConflictError($"Resource binding '{input.Name}' already exists."));

        var entry = new ResourceBinding(
            Name: input.Name,
            Kind: input.Kind,
            Scope: scope,
            ResourceId: scope == ResourceBindingScope.Global ? null : resourceId,
            Value: input.Value,
            SecretId: input.SecretId,
            SecretDeliveryMode: input.SecretDeliveryMode,
            TargetPath: input.TargetPath);

        try
        {
            entry.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<ResourceBindingsResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.ResourceBindings.AddAsync(entry, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        var effective = scope == ResourceBindingScope.Global
            ? entries
            : await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId!.Value, cancellationToken);

        return new ResourceBindingsResult([.. entries], [.. effective]);
    }

    public static async Task<Result<ResourceBindingsResult>> UpdateEntryAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid? resourceId,
        UpdateResourceBindingInputModel input,
        CancellationToken cancellationToken)
    {
        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        var existing = entries.FirstOrDefault(entry => entry.Id == input.Id);
        if (existing is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Resource binding with ID {input.Id} does not exist."));

        if (entries.Any(entry => entry.Id != input.Id && string.Equals(entry.Name, input.Name, StringComparison.Ordinal)))
            return Result.Failure<ResourceBindingsResult>(new ConflictError($"Resource binding '{input.Name}' already exists."));

        var updated = new ResourceBinding(
            Name: input.Name,
            Kind: input.Kind,
            Scope: scope,
            ResourceId: scope == ResourceBindingScope.Global ? null : resourceId,
            Value: input.Value,
            SecretId: input.SecretId,
            SecretDeliveryMode: input.SecretDeliveryMode,
            TargetPath: input.TargetPath)
        {
            Id = existing.Id,
            CreatedAt = existing.CreatedAt,
            UpdatedAt = DateTime.UtcNow
        };

        try
        {
            updated.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<ResourceBindingsResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.ResourceBindings.UpdateAsync(updated, cancellationToken);
        if (existing.SecretId != updated.SecretId)
            await DeleteOrphanedSecretDefinitionAsync(unitOfWork, existing.SecretId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return await GetEntriesResultAsync(unitOfWork, scope, resourceId, cancellationToken);
    }

    public static async Task<Result<ResourceBindingsResult>> DeleteEntryAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid? resourceId,
        Guid id,
        CancellationToken cancellationToken)
    {
        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        var existing = entries.FirstOrDefault(entry => entry.Id == id);
        if (existing is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Resource binding with ID {id} does not exist."));

        await unitOfWork.ResourceBindings.DeleteAsync(id, cancellationToken);
        await DeleteOrphanedSecretDefinitionAsync(unitOfWork, existing.SecretId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return await GetEntriesResultAsync(unitOfWork, scope, resourceId, cancellationToken);
    }

    private static async Task DeleteOrphanedSecretDefinitionAsync(
        IUnitOfWork unitOfWork,
        Guid? secretId,
        CancellationToken cancellationToken)
    {
        if (secretId is null)
            return;

        if (await unitOfWork.SecretDefinitions.IsUsedByResourceBindingAsync(secretId.Value, cancellationToken))
            return;

        await unitOfWork.SecretDefinitions.DeleteAsync(secretId.Value, cancellationToken);
    }

    public static async Task<Result<ResourceBindingsResult>> GetResourceEntriesAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid resourceId,
        Func<Task<bool>> exists,
        CancellationToken cancellationToken)
    {
        if (!await exists())
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"{scope} with ID {resourceId} does not exist."));

        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        var effective = await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId, cancellationToken);
        return new ResourceBindingsResult([.. entries], [.. effective]);
    }

    private static async Task<ResourceBindingsResult> GetEntriesResultAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid? resourceId,
        CancellationToken cancellationToken)
    {
        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        var effective = scope == ResourceBindingScope.Global
            ? entries
            : await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId!.Value, cancellationToken);

        return new ResourceBindingsResult([.. entries], [.. effective]);
    }

    public static async Task<Result<ResourceBindingsResult>> ReplaceEntriesAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid? resourceId,
        IReadOnlyList<ResourceBindingInput> inputs,
        CancellationToken cancellationToken)
    {
        var duplicate = inputs
            .GroupBy(x => x.Name, StringComparer.Ordinal)
            .FirstOrDefault(x => x.Count() > 1);

        if (duplicate is not null)
            return Result.Failure<ResourceBindingsResult>(new BadRequestError($"Resource binding '{duplicate.Key}' is defined more than once."));

        var existingSecretIds = (await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken))
            .Select(entry => entry.SecretId)
            .OfType<Guid>()
            .Distinct()
            .ToArray();

        var entries = inputs
            .Select(input => new ResourceBinding(
                Name: input.Name,
                Kind: input.Kind,
                Scope: scope,
                ResourceId: scope == ResourceBindingScope.Global ? null : resourceId,
                Value: input.Value,
                SecretId: input.SecretId,
                SecretDeliveryMode: input.SecretDeliveryMode,
                TargetPath: input.TargetPath))
            .ToArray();

        try
        {
            foreach (var entry in entries)
                entry.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<ResourceBindingsResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.ResourceBindings.ReplaceEntriesAsync(scope, resourceId, entries, cancellationToken);
        foreach (var secretId in existingSecretIds)
            await DeleteOrphanedSecretDefinitionAsync(unitOfWork, secretId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var effective = scope == ResourceBindingScope.Global
            ? entries
            : await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId!.Value, cancellationToken);

        return new ResourceBindingsResult(entries, [.. effective]);
    }
}
