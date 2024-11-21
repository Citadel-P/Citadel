namespace Infrastructure.EntityFramework.Configurations;

using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

internal sealed class SystemInfoConfiguration : IEntityTypeConfiguration<SystemInfo>
{
    public void Configure(EntityTypeBuilder<SystemInfo> builder)
    {
        builder.ToTable("SystemsInfo");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Properties
        builder.Property(p => p.OperatingSystem).HasMaxLength(128);
        builder.Property(p => p.OperatingSystem).HasMaxLength(128);
        builder.Property(p => p.Architecture).HasMaxLength(128);
        builder.Property(p => p.OSType).HasMaxLength(128);
        builder.Property(p => p.OSVersion).HasMaxLength(128);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);
        builder.Property(p => p.ServerVersion).HasMaxLength(32);

        builder.HasOne(p => p.SwarmInfo).WithOne().HasForeignKey<SwarmInfo>(p => p.SystemInfoId).OnDelete(DeleteBehavior.Cascade);
    }
}