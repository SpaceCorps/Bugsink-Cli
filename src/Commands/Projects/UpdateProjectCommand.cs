using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Projects;

public sealed class UpdateProjectCommand : AsyncCommand<UpdateProjectCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<PROJECT>")]
        [Description("Project id, slug or name")]
        public required string Project { get; init; }

        [CommandOption("--team <TEAM>")]
        [Description("Move the project to this team (UUID or name)")]
        public string? Team { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("New project name")]
        public string? Name { get; init; }

        [CommandOption("--visibility <VISIBILITY>")]
        [Description("joinable, discoverable or team_members")]
        public string? Visibility { get; init; }

        [CommandOption("--alert-on-new-issue <BOOL>")]
        [Description("Send an alert the first time an issue is seen")]
        public bool? AlertOnNewIssue { get; init; }

        [CommandOption("--alert-on-regression <BOOL>")]
        [Description("Send an alert when a resolved issue comes back")]
        public bool? AlertOnRegression { get; init; }

        [CommandOption("--alert-on-unmute <BOOL>")]
        [Description("Send an alert when a muted issue unmutes itself")]
        public bool? AlertOnUnmute { get; init; }

        [CommandOption("--retention <N>")]
        [Description("Maximum number of events to keep for this project")]
        public long? RetentionMaxEventCount { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();

        var body = new Dictionary<string, object?>();
        if (settings.Team is not null) body["team"] = await client.ResolveTeamIdAsync(settings.Team);
        if (settings.Name is not null) body["name"] = settings.Name;
        if (settings.Visibility is not null) body["visibility"] = settings.Visibility;
        if (settings.AlertOnNewIssue is not null) body["alert_on_new_issue"] = settings.AlertOnNewIssue;
        if (settings.AlertOnRegression is not null) body["alert_on_regression"] = settings.AlertOnRegression;
        if (settings.AlertOnUnmute is not null) body["alert_on_unmute"] = settings.AlertOnUnmute;
        if (settings.RetentionMaxEventCount is not null)
            body["retention_max_event_count"] = settings.RetentionMaxEventCount;

        if (body.Count == 0)
            throw new BugsinkApiException("Nothing to update. Pass at least one field option.");

        var projectId = await client.ResolveProjectIdAsync(settings.Project);
        YamlOutput.Write(await client.PatchAsync($"projects/{projectId}/", body));
        Status.Ok($"Project {settings.Project} updated.");
        return 0;
    }
}
