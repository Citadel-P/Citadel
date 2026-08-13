using Hosting.Common.ErrorTypes;
using LightResults;

namespace Domain.Entities.Identity;

public sealed class Actor
{
    public Guid Id { get; private set; }
    public ActorType Type { get; private set; }
    public ActorMetadata ActorMetadata { get; private set; }
    public bool IsEnabled { get; private set; }

    public static Actor Create(ActorType type, ActorMetadata actorMetadata, bool isEnabled = true)
    {
        return new Actor
        {
            Id = Guid.CreateVersion7(),
            Type = type,
            ActorMetadata = actorMetadata,
            IsEnabled = isEnabled
        };
    }

    public Result SetEnabled(bool isEnabled)
    {
        if (Type is not ActorType.User and not ActorType.Team and not ActorType.ServiceAccount)
        {
            return Result.Failure(new BadRequestError($"Actors of type {Type} cannot be enabled or disabled."));
        }

        IsEnabled = isEnabled;
        return Result.Success();
    }

    public static Actor FromPersistence(Guid id, ActorType type, ActorMetadata actorMetadata, bool isEnabled)
    {
        return new Actor
        {
            Id = id,
            Type = type,
            ActorMetadata = actorMetadata,
            IsEnabled = isEnabled
        };
    }
}
public sealed record ActorMetadata(string Name);
