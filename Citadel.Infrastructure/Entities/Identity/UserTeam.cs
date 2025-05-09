namespace Infrastructure.Entities.Identity;

public class UserTeam
{
    public Guid UserId { get; private set; }
    public Guid TeamId { get; private set; }

    public static UserTeam Create(Guid userId, Guid teamId)
    {
        return new UserTeam { UserId = userId, TeamId = teamId };
    }
}