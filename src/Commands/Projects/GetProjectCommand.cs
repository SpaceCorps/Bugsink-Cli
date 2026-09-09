using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Projects;

public sealed class GetProjectCommand : AsyncCommand<GetProjectCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<PROJECT>")]
        [Description("Project id, slug or name")]
        public required string Project { get; init; }

        [CommandOption("--expand-team")]
        [Description("Include the full team object instead of just its UUID")]
        public bool ExpandTeam { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var projectId = await client.ResolveProjectIdAsync(settings.Project);

        var path = $"projects/{projectId}/";
        if (settings.ExpandTeam) path += "?expand=team";

        YamlOutput.Write(await client.GetAsync(path));
        return 0;
    }
}
