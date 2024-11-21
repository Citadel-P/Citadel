namespace Infrastructure.Entities.Identity;

public class Role
{
    /// <summary>
    /// Role Id
    /// </summary>
    public Guid Id { get; private set; }

    /// <summary>
    /// Role name
    /// </summary>
    public string Name { get; private set; }

    /// <summary>
    /// Creation date
    /// </summary>
    public DateTime CreatedAt { get; private set; }

    /// <summary>
    /// Update date
    /// </summary>
    public DateTime UpdatedAt { get; private set; }

    /// <summary>
    /// Permission list
    /// </summary>
    public ICollection<Permission> Permissions { get; } = [];

    public static Role Create(string name) => new () 
    {
        Id = Guid.CreateVersion7(),
        Name = name,
        CreatedAt = DateTime.UtcNow,
    };
    
}