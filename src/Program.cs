using System.Text;
using Bugsink.Console.Commands;
using Bugsink.Console.Commands.Events;
using Bugsink.Console.Commands.Issues;
using Bugsink.Console.Commands.Projects;
using Bugsink.Console.Commands.Releases;
using Bugsink.Console.Commands.Teams;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

// Stacktraces and exception messages carry non-ASCII; the default Windows codepage mangles them.
try
{
    System.Console.OutputEncoding = Encoding.UTF8;
}
catch (IOException)
{
    // No console attached (piped or redirected) — nothing to configure.
}

var app = new CommandApp();

app.Configure(config =>
{
    config.SetApplicationName("bugsink");

    config.AddBranch("issues", issues =>
    {
        issues.SetDescription("Triage issues");
        issues.AddCommand<ListIssuesCommand>("list")
            .WithDescription("List the issues of a project");
        issues.AddCommand<GetIssueCommand>("get")
            .WithDescription("Get one issue by UUID or friendly id");
        issues.AddCommand<IssueStacktraceCommand>("stacktrace")
            .WithDescription("Print the stacktrace of the issue's most recent event");
        issues.AddCommand<ResolveIssueCommand>("resolve")
            .WithDescription("Mark an issue resolved, optionally in the latest or next release");
        issues.AddCommand<ReopenIssueCommand>("reopen")
            .WithDescription("Mark a resolved issue unresolved again");
        issues.AddCommand<MuteIssueCommand>("mute")
            .WithDescription("Mute an issue, for a period or until it gets noisy again");
        issues.AddCommand<UnmuteIssueCommand>("unmute")
            .WithDescription("Unmute an issue");
        issues.AddCommand<CommentIssueCommand>("comment")
            .WithDescription("Add a comment to an issue");
        issues.AddCommand<DeleteIssueCommand>("delete")
            .WithDescription("Delete an issue and its events");
    });

    config.AddBranch("events", events =>
    {
        events.SetDescription("Read the events behind an issue");
        events.AddCommand<ListEventsCommand>("list")
            .WithDescription("List the stored events of an issue");
        events.AddCommand<GetEventCommand>("get")
            .WithDescription("Get one event, including its full payload");
        events.AddCommand<EventStacktraceCommand>("stacktrace")
            .WithDescription("Render one event's stacktrace as Markdown");
    });

    config.AddBranch("projects", projects =>
    {
        projects.SetDescription("Manage projects");
        projects.AddCommand<ListProjectsCommand>("list")
            .WithDescription("List projects, with their DSN and event counts");
        projects.AddCommand<GetProjectCommand>("get")
            .WithDescription("Get one project by id, slug or name");
        projects.AddCommand<CreateProjectCommand>("create")
            .WithDescription("Create a project in a team");
        projects.AddCommand<UpdateProjectCommand>("update")
            .WithDescription("Update a project's name, team, visibility, alerts or retention");
    });

    config.AddBranch("teams", teams =>
    {
        teams.SetDescription("Manage teams");
        teams.AddCommand<ListTeamsCommand>("list")
            .WithDescription("List all teams");
        teams.AddCommand<GetTeamCommand>("get")
            .WithDescription("Get one team by UUID or name");
        teams.AddCommand<CreateTeamCommand>("create")
            .WithDescription("Create a team");
        teams.AddCommand<UpdateTeamCommand>("update")
            .WithDescription("Update a team's name or visibility");
    });

    config.AddBranch("releases", releases =>
    {
        releases.SetDescription("Manage releases");
        releases.AddCommand<ListReleasesCommand>("list")
            .WithDescription("List the releases of a project");
        releases.AddCommand<GetReleaseCommand>("get")
            .WithDescription("Get one release by UUID");
        releases.AddCommand<CreateReleaseCommand>("create")
            .WithDescription("Create a release for a project");
    });

    config.AddCommand<AgentReadmeCommand>("agent-readme")
        .WithDescription("Print the operating manual for an LLM agent");

    // One line on stderr beats a stack trace: the caller is usually a script or an agent that only
    // needs to know what went wrong and that the exit code is non-zero.
    config.SetExceptionHandler((exception, _) =>
    {
        Status.Fail(exception.Message);
        return 1;
    });
});

return app.Run(args);
