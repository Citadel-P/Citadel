namespace Infrastructure.EntityFramework.JoiningTables;

public class UserTeam
{
    public Guid UserId { get; set; }
    public Guid TeamId { get; set; }
}