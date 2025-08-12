namespace Domain.Entities.Identity;

public class Role
{
    /// <summary>
    /// Role ID
    /// </summary>
    public Guid Id { get; private set; }

    /// <summary>
    /// Role name
    /// </summary>
    public string Name { get; private set; } = null!;

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

    public static Role Create(string name, Guid? id = null, DateTime? createdAt = null) => new () 
    {
        Name = name,
        Id = id ?? Guid.CreateVersion7(),
        CreatedAt = createdAt ?? DateTime.UtcNow,
    };
    
}