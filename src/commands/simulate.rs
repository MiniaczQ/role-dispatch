use crate::model::{Job, load_jobs};
use serenity::{
    framework::standard::{
        macros::command,
        Args, CommandResult,
    },
    model::prelude::*,
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use super::{roll::decide_pairings};

#[command]
#[aliases("sim")]
#[only_in(guilds)]
#[description("Simulate how many of each role will be required for specific player count.")]
async fn simulate(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
    let users_roles = simulate_roles(&jobs, player_count);
    let mut assigned = decide_pairings(&mut jobs, &users_roles).unwrap();
    let roles = msg.guild_id.unwrap().roles(&ctx.http).await.unwrap();

    let mut content = String::new();
    content.write_fmt(format_args!("Simulation for {} players:\n", player_count)).ok();
    assigned.sort_keys();
    assigned.into_iter().for_each(|(role_id, players)| {
        content
            .write_fmt(format_args!(
                "**{}**: {}\n",
                roles.get(&role_id).unwrap().name,
                players.len()
            ))
            .ok();
    });

    msg.channel_id
        .say(ctx, content)
        .await
        .ok();

    Ok(())
}

fn simulate_roles(jobs: &HashMap<u64, Job>, count: u16) -> HashMap<UserId, HashSet<RoleId>> {
    let mut users_roles = HashMap::new();
    for i in 0..count {
        users_roles.insert(
            UserId::from(i as u64),
            jobs.iter().map(|(role_id, _job)| RoleId(*role_id)).collect(),
        );
    }
    users_roles
}
