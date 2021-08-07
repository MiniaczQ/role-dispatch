use crate::model::{Job, QUALIFICATION_PREFIX, load_jobs, save_jobs};
use indexmap::{IndexMap, IndexSet};
use rand::{distributions::{Uniform, WeightedIndex}, prelude::Distribution};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use std::{collections::HashMap, fmt::Write, time::Duration};

#[command]
pub async fn roll(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let anchor = msg
        .reply(ctx, "React with the dice to randomize roles!")
        .await
        .unwrap();
    anchor
        .react(ctx, ReactionType::Unicode("🎲".to_owned()))
        .await
        .unwrap();
    anchor
        .react(ctx, ReactionType::Unicode("❌".to_owned()))
        .await
        .unwrap();

    let mut response: Option<Message> = None;

    loop {
        let result = anchor
            .await_reaction(&ctx)
            .timeout(Duration::from_secs(60 * 60))
            .await;

        if let Some(msg) = response {
            msg.delete(ctx).await.unwrap();
            response = None;
        }

        if let Some(reaction) = result {
            let reaction = reaction.as_inner_ref();
            let emoji = &reaction.emoji;
            match emoji.as_data().as_str() {
                "🎲" => {
                    response = try_assigning(ctx, &msg, &anchor.channel_id).await;
                }
                "❌" => break,
                _ => (),
            };
            reaction.delete(ctx).await.unwrap();
        } else {
            break;
        };
    }

    anchor.delete(ctx).await.unwrap();
    if let Some(msg) = response {
        msg.delete(ctx).await.unwrap();
    }

    Ok(())
}

/// Decide role for every participating player
async fn try_assigning(ctx: &Context, msg: &Message, channel_id: &ChannelId) -> Option<Message> {
    let guild = &msg.guild(ctx).await.unwrap();
    msg.delete(ctx).await.ok();
    if let Some(vc_id) = get_vc(guild, &msg.author.id).await {
        let jobs = get_jobs(guild);
        let players = get_players(guild, vc_id, &jobs);
        let pairings = decide_pairings(&jobs, &players);
        return Some(display_pairings(ctx, guild, channel_id, &jobs, pairings).await);
    }
    None
}

/// Returns the voice channel id of the user.
async fn get_vc(guild: &Guild, author_id: &UserId) -> Option<ChannelId> {
    let result = guild.voice_states.get(author_id);
    if let Some(voice_state) = result {
        return voice_state.channel_id;
    }
    None
}

/// Returns all players in a voice channel.
fn get_players_in_vc(guild: &Guild, voice_channel_id: ChannelId) -> Vec<UserId> {
    let mut user_ids: Vec<UserId> = Vec::new();
    for (user_id, voice_state) in guild.voice_states.iter() {
        if let Some(other_id) = voice_state.channel_id {
            if other_id == voice_channel_id {
                user_ids.push(user_id.clone());
            }
        }
    }
    user_ids
}

/// Returns all players roles.
fn get_player_roles(guild: &Guild, player_ids: Vec<UserId>) -> HashMap<UserId, Vec<RoleId>> {
    let mut players_roles = HashMap::new();
    let members = &guild.members;
    for player_id in player_ids {
        let member = members.get(&player_id).unwrap();
        players_roles.insert(player_id, member.roles.clone());
    }
    players_roles
}

/// Removes qualifications that don't have jobs.
fn remove_irrelevant_qualifications(
    players: &mut HashMap<UserId, Vec<RoleId>>,
    jobs: &HashMap<RoleId, Job>,
) {
    for (_id, roles) in players.iter_mut() {
        roles.retain(|e| jobs.get(e).is_some());
    }
}

/// All server roles that start with the qualified prefix.
fn get_qualifying_roles(guild: &Guild) -> HashMap<RoleId, String> {
    let mut role_names = HashMap::new();
    for (role_id, role) in guild.roles.iter() {
        if role.name.starts_with(QUALIFICATION_PREFIX) {
            role_names.insert(
                *role_id,
                role.name[(QUALIFICATION_PREFIX.len())..].to_string(),
            );
        }
    }
    role_names
}

/// Jobs mapped by their corresponding role id.
fn get_jobs(guild: &Guild) -> HashMap<RoleId, Job> {
    let mut qualifying_roles = get_qualifying_roles(guild);
    let jobs_vec = load_jobs();
    let mut jobs_mapped = HashMap::new();
    for job in jobs_vec {
        let found = qualifying_roles.iter().find_map(|(id, name)| {job.name.eq(name).then_some(*id)});
        if let Some(id) = found {
            qualifying_roles.remove(&id);
            jobs_mapped.insert(id, job);
        }
    }
    jobs_mapped
}

/// Players with only useful qualifications.
fn get_players(
    guild: &Guild,
    voice_channel_id: ChannelId,
    jobs: &HashMap<RoleId, Job>,
) -> HashMap<UserId, Vec<RoleId>> {
    let player_ids = get_players_in_vc(&guild, voice_channel_id);
    let mut players = get_player_roles(&guild, player_ids);
    remove_irrelevant_qualifications(&mut players, jobs);
    players
}

#[derive(Debug)]
struct JobExt {
    at_least: u32,
    assigned: u32,
    players: IndexSet<UserId>,
}

/// Decide pairings.
/// Start with Hungarian Algorithm to fill the minimums.
/// Continue by random assignment.
fn decide_pairings(
    jobs: &HashMap<RoleId, Job>,
    players: &HashMap<UserId, Vec<RoleId>>,
) -> Vec<(RoleId, UserId)> {
    // Left players with available jobs
    // Left jobs with available players, how many more players are required
        // Start with minimal slots
        // Remove jobs with maxed slots
        // Normalize proportions for the remaining jobs
        // Calculate fractional amounts
            // If exceeding max bound, redistribute the reminder to other jobs
        // Round fractions to the bottom for the new minimal amount
            // Distribute the remaining players with the fractional part as the weight
            // or
            // Distribute starting from the highest remainign fractional part, until we run out
        // Remove jobs with maxed slots

    // Loop until all spots used:
        // Find job with least players
        // Choose it a (semi)random player
        // Update jobs-players and players-jobs
        // Remember the pairing in a map<job, vec<player>>

    
    
    // Don't fuck it up

    let mut pairings: Vec<(RoleId, UserId)> = Vec::new();
    let mut left_jobs = jobs.iter().map(|(role_id, job)| {
        (role_id, (job, JobExt{
            at_least: job.minimum,
            assigned: 0,
            players: players.iter().filter_map(|(player_id, roles)| {
                roles.iter().find(|id| (*id).eq(role_id)).is_some().then_some(*player_id)
            }).collect(),
        }))
    }).collect::<IndexMap<_, _>>();
    let mut left_players = players.iter().map(|e| {(e.0, e.1.iter().copied().collect::<IndexSet<_>>())}).collect::<IndexMap<_, _>>();
    let mut rng = rand::thread_rng();
    // Fill minimums
    let minimum: u32 = left_jobs.iter().map(|e| {e.1.1.at_least}).sum();
    for _ in (0..minimum).rev() {
        let e = left_jobs.iter_mut().filter(|e| {e.1.1.at_least > 0}).min_by_key(|e| {e.1.1.players.len()}).unwrap();
        e.1.1.at_least -= 1;
        e.1.1.assigned += 1;
        let job_id = **e.0;
        let left = e.1.1.players.len();
        let player = *e.1.1.players.get_index(Uniform::from(0..left).sample(&mut rng) as usize).unwrap();
        for job in left_jobs.iter_mut() {
            job.1.1.players.remove(&player);
        }
        left_players.remove(&player);
        pairings.push((job_id, player));
    }
    remove_maxed_jobs(&mut left_jobs, &mut left_players);
    // Use up the rest of players
    for left in (0..left_players.len()).rev() {
        let player = left_players.get_index(Uniform::from(0..(left+1)).sample(&mut rng) as usize).unwrap();
        let dist = WeightedIndex::new(player.1.iter().map(|e| {left_jobs.get(e).unwrap().0.weight})).unwrap();
        let job_id = *player.1.get_index(dist.sample(&mut rng)).unwrap();
        let e = left_jobs.get_mut(&job_id).unwrap();
        e.1.assigned += 1;
        for job in left_jobs.iter_mut() {
            job.1.1.players.remove(*player.0);
        }
        let player_id = **player.0;
        left_players.remove(&player_id);
        pairings.push((job_id, player_id));
        remove_maxed_jobs(&mut left_jobs, &mut left_players);
    }
    pairings
}

/*
fn remove_maxed_jobs(
    left_jobs: &mut IndexMap<&RoleId, (&Job, JobExt)>,
    left_players: &mut IndexMap<&UserId, IndexSet<RoleId>>,
) {
    for e in left_jobs.iter() {
        if e.1.1.assigned >= e.1.0.maximum {
            for player in left_players.iter_mut() {
                player.1.remove(*e.0);
            }
        }
    }
    left_jobs.retain(|_, e| {e.1.assigned < e.0.maximum});
}*/

/// Displays the distribution of roles.
/*
async fn display_pairings(
    ctx: &Context,
    guild: &Guild,
    channel_id: &ChannelId,
    jobs: &HashMap<RoleId, Job>,
    mut pairings: Vec<(RoleId, UserId)>,
) -> Message {
    let channel = guild.channels.get(channel_id).unwrap();
    let mut content = String::new();
    pairings.sort_by(|a, b| a.0.cmp(&b.0));
    for (role_id, user_id) in pairings {
        content += &format!("<@{}>: {}\n", user_id, jobs.get(&role_id).unwrap().name);
    }

    channel
        .send_message(ctx, |m| m.content(content))
        .await
        .unwrap()
}*/

#[command]
pub async fn modify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    let response = match parse_args_to_job(args) {
        Ok(job) => {
            let mut jobs = load_jobs();
            let job_name = job.name.clone();
            let response = match jobs.insert(job.name.to_ascii_lowercase(), job) {
                Some(previous_job) => {
                    let role = guild.role_by_name(&previous_job.name).unwrap();
                    role.edit(ctx, |e| {
                        e.name(job_name)
                    }).await.ok();
                    "Role modified succesfully!".to_owned()
                },
                None => {
                    guild.create_role(ctx, |e| {
                        e.name(job_name)
                    }).await.ok();
                    "Role added succesfully!".to_owned()
                },
            };
            save_jobs(jobs);
            response
        },
        Err(e) => e,
    };

    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel.send_message(ctx, |m| {
        m.content(response)
    }).await.ok();

    Ok(())
}

fn parse_args_to_job(mut args: Args) -> Result<Job, String> {
    args.trimmed().quoted();
    let name = match args.single::<String>() {
        Ok(name) => name,
        Err(_) => return Err("Invalid name".to_owned()),
    };
    let minimum = match args.single::<u32>() {
        Ok(name) => name,
        Err(_) => return Err("Invalid minimum (non-negative integer)".to_owned()),
    };
    let maximum = match args.single::<u32>() {
        Ok(name) => name,
        Err(_) => return Err("Invalid maximum (non-negative integer)".to_owned()),
    };
    let proportion = match args.single::<u64>() {
        Ok(proportion) => proportion,
        Err(_) => return Err("Invalid proportion (non-negative integer)".to_owned()),
    };
    return Ok(Job {
        name,
        minimum,
        maximum,
        proportion,
    })
}

#[command]
pub async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    args.trimmed().quoted();
    let response = match args.single::<String>() {
        Ok(name) => {
            let mut jobs = load_jobs();
            let response = match jobs.remove(&name.to_lowercase()) {
                Some(job) => {
                    let role = guild.role_by_name(&job.name).unwrap();
                    guild.delete_role(ctx, role.id).await.ok();
                    "Role removed succesfully!".to_owned()
                },
                None => "No role with that name exists.".to_owned(),
            };
            save_jobs(jobs);
            response
        },
        Err(_) => "Invalid name".to_owned(),
    };

    let guild = msg.guild(ctx).await.unwrap();
    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel.send_message(ctx, |m| {
        m.content(response)
    }).await.ok();

    Ok(())
}

#[command]
pub async fn show(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let jobs = load_jobs();
    let mut response = String::new();
    response.write_str("**Existing roles:**\n\n").ok();
    for (_key, job) in jobs {
        response.write_fmt(format_args!(
            "***{}***\n> **Minimum**: {}\n> **Maximum**: {}\n> **Proportion**: {}\n\n",
            job.name, job.minimum, job.maximum, job.proportion
        )).ok();
    }

    let guild = msg.guild(ctx).await.unwrap();
    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel.send_message(ctx, |m| {
        m.content(response)
    }).await.ok();

    Ok(())
}

#[command]
pub async fn grant(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    let jobs = load_jobs();
    let mut member = msg.member(ctx).await.unwrap();
    let mut invalid = Vec::<String>::new();
    args.trimmed().quoted();
    for _ in 0..args.len() {
        let name = args.single::<String>().unwrap();
        match jobs.get(&name.to_ascii_lowercase()) {
            Some(_) => {
                match guild.role_by_name(&name) {
                    Some(role) => member.add_role(ctx, role.id).await.unwrap(),
                    None => invalid.push(name),
                }
            }
            None => {
                invalid.push(name);
            }
        }
    }

    let response = match invalid.len() {
        0 => "Roles were granted succesfully!".to_owned(),
        _ => {
            let mut response = String::new();
            for s in invalid.iter() {
                response.write_fmt(format_args!(
                    "{} is not a valid role.\n",
                    s
                )).ok();
            }
            if args.len() != invalid.len() {
                response.write_str("The rest of the roles were granted succesfully!").ok();
            }
            response
        }
    };
    
    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel.send_message(ctx, |m| {
        m.content(response)
    }).await.ok();

    Ok(())
}

#[command]
pub async fn revoke(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = msg.guild(ctx).await.unwrap();
    let jobs = load_jobs();
    let mut member = msg.member(ctx).await.unwrap();
    let mut invalid = Vec::<String>::new();
    args.trimmed().quoted();
    for _ in 0..args.len() {
        let name = args.single::<String>().unwrap();
        match jobs.get(&name.to_ascii_lowercase()) {
            Some(_) => {
                match guild.role_by_name(&name) {
                    Some(role) => member.remove_role(ctx, role.id).await.unwrap(),
                    None => invalid.push(name),
                }
            },
            None => {
                invalid.push(name);
            }
        }
    }

    let response = match invalid.len() {
        0 => "Roles were revoked succesfully!".to_owned(),
        _ => {
            let mut response = String::new();
            for s in invalid.iter() {
                response.write_fmt(format_args!(
                    "{} is not a valid role.\n",
                    s
                )).ok();
            }
            if args.len() != invalid.len() {
                response.write_str("The rest of the roles were revoked succesfully!").ok();
            }
            response
        }
    };
    
    let channels = guild.channels(ctx).await.unwrap();
    let channel = channels.get(&msg.channel_id).unwrap();
    channel.send_message(ctx, |m| {
        m.content(response)
    }).await.ok();

    Ok(())
}