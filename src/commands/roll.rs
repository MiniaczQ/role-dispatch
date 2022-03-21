use crate::model::{load_jobs, Job};
use indexmap::{IndexMap, IndexSet};
use rand::{distributions::Uniform, prelude::Distribution};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use super::util::{get_callers_vc, get_members_in_vc, get_users_roles, remove_irrelevant_qualifications, remove_excluded};

#[command]
#[aliases("r")]
#[only_in(guilds)]
#[description("Assigns roles to all players in the caller's voice channel.")]
pub async fn roll(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    try_assigning(&ctx, &msg).await;

    Ok(())
}

/// Decide role for every participating player
async fn try_assigning(ctx: &Context, msg: &Message) -> Option<Message> {
    if let Some(voice_channel_id) = get_callers_vc(ctx, msg).await {
        let members = get_members_in_vc(ctx, msg, voice_channel_id).await;
        let mut users_roles = get_users_roles(members);
        remove_excluded(&mut users_roles);
        let mut jobs = load_jobs();
        remove_irrelevant_qualifications(&mut users_roles, &jobs);
        let assigned = decide_pairings(&mut jobs, &users_roles);
        return Some(display_pairings(ctx, msg, assigned).await);
    }
    None
}

#[derive(Debug)]
struct JobExt {
    at_least: u32,
    assigned: u32,
    players: IndexSet<UserId>,
}

/// Decide pairings.
/// Decide how many of each role based on the proportions and limits
/// Finish by applying last step of Hungarian Algorithm.
/// Returns error if there was a problem during assignment.
pub fn decide_pairings(
    jobs: &mut HashMap<u64, Job>,
    users_roles: &HashMap<UserId, HashSet<RoleId>>,
) -> Result<IndexMap<RoleId, Vec<UserId>>, RoleId> {
    /// Variables required for assigning jobs.
    #[derive(Debug)]
    struct AssigningJob {
        needed: u16,
        leftovers: f64,
        players: IndexSet<UserId>,
    }

    /// Variables required for assigning players.
    #[derive(Debug)]
    struct AssigningPlayer {}

    /// Assigns a player to a job.
    /// Update all the variables.
    /// Remove used players and jobs.
    fn assign(
        assigned: &mut IndexMap<RoleId, Vec<UserId>>,
        left_jobs: &mut IndexMap<RoleId, AssigningJob>,
        left_players: &mut IndexMap<UserId, AssigningPlayer>,
        role_id: RoleId,
        user_id: UserId,
    ) {
        // Update `assigned`
        if let Some(vec) = match assigned.get_mut(&role_id) {
            Some(vec) => {
                vec.push(user_id);
                None
            }
            None => {
                let mut vec = Vec::new();
                vec.push(user_id);
                Some(vec)
            }
        } {
            assigned.insert(role_id, vec);
        };
        // Update `left_jobs`
        let job = left_jobs.get_mut(&role_id).unwrap();
        job.needed -= 1;
        if job.needed <= 0 {
            left_jobs.remove(&role_id);
        }
        left_jobs.iter_mut().for_each(|(_role_id, job)| {
            job.players.remove(&user_id);
        });
        // Update `left_players`
        left_players.remove(&user_id);
    }

    // Initialize assignment variables
    let amount = users_roles.len() as u16;
    let mut used_amount: u16 = 0;
    let mut assigned: IndexMap<RoleId, Vec<UserId>> = IndexMap::new();
    let mut left_jobs: IndexMap<RoleId, AssigningJob> = jobs
        .iter_mut()
        .map(|(role_id, job)| {
            let interpolated = job.interpolate(amount);
            let full = interpolated.trunc() as u16;
            used_amount += full;
            (
                RoleId(*role_id),
                AssigningJob {
                    needed: full,
                    leftovers: interpolated.fract(),
                    players: users_roles
                        .iter()
                        .filter_map(|(user_id, roles)| {
                            roles.get(&RoleId(*role_id)).is_some().then_some(*user_id)
                        })
                        .collect(),
                },
            )
        })
        .collect();
    let mut for_assignment: IndexMap<RoleId, AssigningJob> = IndexMap::new();
    let rng = &mut rand::thread_rng();

    // Distribute based on the remaining fractions
    // Check if too many players
    for _ in 0..(amount - used_amount) {
        let (role_id, _job) = left_jobs
            .iter_mut()
            .max_by(|(_role_id_a, job_a), (_role_id_b, job_b)| {
                job_a.leftovers.total_cmp(&job_b.leftovers)
            })
            .unwrap();
        let role_id = *role_id;
        let mut job = left_jobs.remove(&role_id).unwrap();
        job.leftovers = 0.;
        job.needed += 1;
        for_assignment.insert(role_id, job);
    }
    left_jobs.into_iter().for_each(|(role_id, mut job)| {
        job.leftovers = 0.;
        for_assignment.insert(role_id, job);
    });
    let mut left_jobs = for_assignment;
    // Initialize more assignment variables
    let mut left_players: IndexMap<UserId, AssigningPlayer> = users_roles
        .iter()
        .map(|(user_id, _roles)| (*user_id, AssigningPlayer {}))
        .collect();
    // Assign players to roles
    for _ in 0..left_players.len() {
        let (role_id, job) = left_jobs
            .iter()
            .filter(|(_role_id, job)| job.needed > 0)
            .min_by_key(|(_role_id, job)| job.players.len())
            .unwrap();
        let role_id = *role_id;
        let available = job.players.len();
        // Check if any players are available
        if available == 0 {
            return Err(role_id);
        }
        let user_id = *job
            .players
            .get_index(Uniform::from(0..available).sample(rng))
            .unwrap();
        assign(
            &mut assigned,
            &mut left_jobs,
            &mut left_players,
            role_id,
            user_id,
        );
    }
    Ok(assigned)
}

/// Displays the distribution of roles or the error.
async fn display_pairings(
    ctx: &Context,
    msg: &Message,
    assigned: Result<IndexMap<RoleId, Vec<UserId>>, RoleId>,
) -> Message {
    let roles = msg.guild_id.unwrap().roles(&ctx.http).await.unwrap();
    let content = match assigned {
        Ok(mut assigned) => {
            match assigned.len() {
                0 => "No players.".to_owned(),
                _ => {
                    let mut content = String::new();
                    assigned.sort_keys();
                    assigned.into_iter().for_each(|(role_id, players)| {
                        content
                            .write_fmt(format_args!("{}:\n", roles.get(&role_id).unwrap().name))
                            .ok();
                        players.into_iter().for_each(|user_id| {
                            content.write_fmt(format_args!("- <@{}>,\n", user_id)).ok();
                        });
                        content.write_str("\n").ok();
                    });
                    content
                },
            }
        }
        Err(role_id) => format!("Not enough players qualified for role '{}'.", roles.get(&role_id).unwrap().name),
    };

    msg.channel_id.say(&ctx.http, content).await.unwrap()
}
