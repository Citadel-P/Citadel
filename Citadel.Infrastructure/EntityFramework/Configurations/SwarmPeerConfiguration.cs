using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class SwarmPeerConfiguration : IEntityTypeConfiguration<SwarmPeer>
{
    public void Configure(EntityTypeBuilder<SwarmPeer> builder)
    {
        builder.ToTable("SwarmsPeer");

        // Pk & Indexes
        builder.HasKey(p => p.Id);

        // Properties
        builder.Property(p => p.Addr).HasMaxLength(64);
        builder.Property(p => p.NodeID).HasMaxLength(64);
    }
}