use crate::model::{load_jobs, save_jobs, Job};
use indexmap::IndexMap;
use serenity::{
    framework::standard::{
        macros::command,
        Args, CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::fmt::Write;

#[command]
#[sub_commands(add, remove, list)]
#[only_in(guilds)]
#[description("Management of roles.")]
async fn roles(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.channel_id
        .say(ctx, "Invalid subcommand.".to_owned())
        .await
        .ok();

    Ok(())
}

#[command]
#[aliases("+")]
#[only_in(guilds)]
#[description("Add a new role by specifying it's name.")]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() != 1 {
        msg.channel_id
            .say(ctx, "Invalid amount of arguments.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    args.trimmed().quoted();
    let name = args.single::<String>().unwrap();
    let partial_guild = msg
        .guild_id
        .unwrap()
        .to_partial_guild(&ctx.http)
        .await
        .unwrap();

    if let Some(_) = partial_guild.role_by_name(&name) {
        msg.channel_id
            .say(ctx, "Role with that name already exists.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let mut jobs = load_jobs();
    let points = match jobs.iter().next() {
        Some((_, job)) => job
            .points
            .iter()
            .map(|(player_count, _)| (*player_count, 0))
            .collect(),
        None => IndexMap::new(),
    };
    let job = Job { points };

    let result = partial_guild
        .create_role(&ctx.http, |r| r.name(name))
        .await;

    if let Err(_) = result {
        msg.channel_id
            .say(ctx, "Failed to create role.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let role = result.unwrap();
    jobs.insert(role.id.0, job);
    save_jobs(jobs);

    msg.channel_id
        .say(ctx, "Role added succesfully.".to_owned())
        .await
        .ok();
    Ok(())
}

#[command]
#[aliases("-")]
#[only_in(guilds)]
#[description("Remove a role by specifying it's name.")]
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() != 1 {
        msg.channel_id
            .say(ctx, "Invalid amount of arguments.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    args.trimmed().quoted();
    let name = args.single::<String>().unwrap();
    let mut jobs = load_jobs();
    let partial_guild = msg
        .guild_id
        .unwrap()
        .to_partial_guild(&ctx.http)
        .await
        .unwrap();
    let result = partial_guild.role_by_name(&name);

    if let None = result {
        msg.channel_id
            .say(ctx, "Role doesn't exist.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let role = result.unwrap();
    if let None = jobs.remove(&role.id.0) {
        msg.channel_id
            .say(ctx, "Role doesn't exist.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    save_jobs(jobs);

    msg.channel_id
        .say(ctx, "Role removed succesfully.".to_owned())
        .await
        .ok();
    Ok(())
}

#[command]
#[aliases("l")]
#[only_in(guilds)]
#[description("List all existing roles.")]
async fn list(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.len() != 0 {
        msg.channel_id
            .say(ctx, "Invalid amount of arguments".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let jobs = load_jobs();

    if jobs.len() == 0 {
        msg.channel_id
            .say(ctx, "No existing roles.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let mut response = String::new();
    let partial_guild = msg
        .guild_id
        .unwrap()
        .to_partial_guild(&ctx.http)
        .await
        .unwrap();

    response.write_str("Existing roles:\n").ok();
    for (role_id, _job) in jobs {
        let role = partial_guild.roles.get(&RoleId(role_id)).unwrap();
        response.write_fmt(format_args!("- {},\n", role.name)).ok();
    }
    msg.channel_id.say(ctx, response).await.ok();

    Ok(())
}

