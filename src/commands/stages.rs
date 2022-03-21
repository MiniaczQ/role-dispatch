use crate::model::{load_jobs, save_jobs};
use serenity::{
    framework::standard::{
        macros::command,
        Args, CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::{collections::HashMap, fmt::Write};

#[command]
#[only_in(guilds)]
#[sub_commands(add, remove, list)]
#[description("Management of role distributions.")]
async fn stages(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.channel_id
        .say(ctx, "Invalid subcommand.".to_owned())
        .await
        .ok();

    Ok(())
}

#[command]
#[aliases("+")]
#[only_in(guilds)]
#[description("Add a stage for specific player count. Specify the role name, followed by amount of players. Repeat for all non-zero roles. The amount of players this will apply to will be specified by the sum of all players.")]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult{
    let argc = args.len();
    if (argc == 0) || (argc % 2 != 0) {
        msg.channel_id
            .say(ctx, "Invalid amount of arguments.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    args.trimmed().quoted();
    let mut jobs = load_jobs();
    let mut total: u16 = 0;
    let mut pairs: HashMap<u64, u16> = HashMap::new();
    let partial_guild = msg
        .guild_id
        .unwrap()
        .to_partial_guild(&ctx.http)
        .await
        .unwrap();

    for _ in 0..(argc / 2) {
        let name = args.single::<String>().unwrap();
        let valid_amount = args.single::<u16>().ok();
        let valid_job = partial_guild.role_by_name(&name).and_then(|role| {
            jobs.get(&role.id.0).and_then(|_job| Some(role.id))
        });
        match (valid_job, valid_amount) {
            (Some(role_id), Some(amount)) => {
                total += amount;
                pairs.insert(role_id.0, amount);
            }
            (None, _) => {
                msg.channel_id
                    .say(ctx, format!("Invalid role name '{}'.", name))
                    .await
                    .ok();
                return Ok(());
            }
            (_, None) => {
                msg.channel_id
                    .say(ctx, format!("Invalid amount for role '{}'.", name))
                    .await
                    .ok();
                return Ok(());
            }
        }
    }

    for (role_id, job) in jobs.iter_mut() {
        let amount = match pairs.get(role_id) {
            Some(amount) => *amount,
            None => 0,
        };
        job.points.insert(total, amount);
    }

    save_jobs(jobs);

    msg.channel_id
        .say(ctx, "Stage succesfully added.".to_owned())
        .await
        .ok();
    Ok(())
}

#[command]
#[aliases("-")]
#[only_in(guilds)]
#[description("Remove a stage for specific player count.")]
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult{
    if args.len() != 1 {
        msg.channel_id
            .say(ctx, "Invalid amount of arguments.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    args.trimmed().quoted();
    let result = args.single::<u16>();

    if let Err(_) = result {
        msg.channel_id
            .say(ctx, "Invalid player count.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let player_count = result.unwrap();
    let mut jobs = load_jobs();
    jobs.iter_mut().for_each(|(_role_id, job)| {
        job.points.remove(&player_count);
    });
    save_jobs(jobs);

    msg.channel_id
        .say(ctx, "Stage succesfully removed.".to_owned())
        .await
        .ok();
    Ok(())
}

#[command]
#[aliases("l")]
#[only_in(guilds)]
#[description("List all stages.")]
async fn list(ctx: &Context, msg: &Message, args: Args) -> CommandResult{
    if args.len() != 0 {
        msg.channel_id
            .say(ctx, "Invalid amount of arguments".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let mut jobs = load_jobs();
    jobs.iter_mut().for_each(|(_role_id, job)| {
        job.points.sort_keys();
    });
    if jobs.len() == 0 {
        msg.channel_id
            .say(ctx, "No roles to list.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let some_job = jobs.iter().next().unwrap().1;
    if some_job.points.len() == 0 {
        msg.channel_id
            .say(ctx, "No stages to list.".to_owned())
            .await
            .ok();
        return Ok(());
    }

    let roles = msg.guild_id.unwrap().roles(&ctx.http).await.unwrap();
    let max_name_len = jobs.iter().map(|(role_id, _job)| {roles.get(&RoleId(*role_id)).unwrap().name.chars().count()}).max().unwrap();
    let max_stage_len = some_job.points.keys().max().unwrap().to_string().chars().count();

    let mut content = String::new();
    content.write_str("```").ok();
    // Header
    content.write_fmt(format_args!("{:<width$} ", "", width=max_name_len)).ok();
    for (player_amount, _role_amount) in some_job.points.iter() {
        content.write_fmt(format_args!("{:>width$} ", player_amount, width=max_stage_len)).ok();
    }
    // Rows
    content.write_str("\n").ok();
    for (role_id, job) in jobs {
        let name = &roles.get(&RoleId(role_id)).unwrap().name;
        content.write_fmt(format_args!("{:<width$} ", name, width=max_name_len)).ok();
        for (_player_amount, role_amount) in job.points {
            content.write_fmt(format_args!("{:>width$} ", role_amount, width=max_stage_len)).ok();
        }
        content.write_str("\n").ok();
    }
    content.write_str("```").ok();

    msg.channel_id
        .say(ctx, content)
        .await
        .ok();

    Ok(())
}
