namespace Infrastructure.Entities;

public class Platform
{
    public Guid Id { get; private set; }
    public string Name { get; private set; }
    public string Address { get; private set; }

    public SystemInfo SystemInfo { get; private set; }
    public ICollection<PlatformStat> Stats { get; private set; } = [];

    public static Platform Create(
        string name,
        string address,
        SystemInfo systemInfo = null,
        IEnumerable<PlatformStat> stats = null)
        => new () { 
            Id = Guid.CreateVersion7(),
            Name = name, 
            Address = address, 
            SystemInfo = systemInfo ,
            Stats = stats?.ToList() ?? []
        };
    
    public Platform Update(string name, string address)
    {
        Name = name;
        Address = address;
        return this;
    }
}