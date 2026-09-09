using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Releases;

public sealed class ListReleasesCommand : AsyncCommand<ProjectSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, ProjectSettings settings)
    {
        var client = settings.CreateClient();
        var projectId = await client.ResolveProjectIdAsync(settings.Project);
        YamlOutput.Write(await client.GetPagedAsync($"releases/?project={projectId}", settings.EffectiveLimit));
        return 0;
    }
}
