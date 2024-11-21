using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal class ContainerStatConfiguration : IEntityTypeConfiguration<ContainerStat>
{
    public void Configure(EntityTypeBuilder<ContainerStat> builder)
    {
        builder.ToTable("ContainerStats");
        builder.HasKey(p => p.Id);
    }
}
