use crate::model::{load_jobs, Job, QUALIFICATION_PREFIX};
use indexmap::{IndexMap, IndexSet};
use rand::{distributions::{Uniform, WeightedIndex}, prelude::Distribution};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use std::{collections::HashMap, time::Duration};

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
}

/// Displays the distribution of roles.
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
}