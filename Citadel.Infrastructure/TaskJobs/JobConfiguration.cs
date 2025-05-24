namespace Infrastructure.TaskJobs;

public class JobConfiguration
{
    public int SystemInfoInterval { get; set; } = 15;

    public int ContainersInfoInterval { get; set; } = 15;
}
