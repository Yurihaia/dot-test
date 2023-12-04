use std::collections::HashSet;

use xivc_core::{
    status_effect,
    world::{
        status::{StatusEffect, StatusInstance, StatusSnapshot},
        ActorId,
    }, math::{HitTypeHandle, XivMath, PlayerStats, WeaponInfo, PlayerInfo, ActionStat, SpeedStat, EotSnapshot}, enums::{Clan, Job},
};

fn main() {
    use HitTypeHandle::*;

    let math = XivMath::new(
        PlayerStats {
            str: 3229,
            vit: 3480,
            dex: 0,
            int: 0,
            mnd: 0,
            det: 1590,
            crt: 2298,
            dh: 1485,
            sks: 542,
            sps: 0,
            ten: 0,
            pie: 0,
        },
        WeaponInfo {
            phys_dmg: 130,
            magic_dmg: 0,
            auto: 12133,
            delay: 280,
        },
        PlayerInfo {
            clan: Clan::Moon,
            job: Job::DRG,
            lvl: 90,
        },
    );

    const PS: StatusEffect = status_effect!(
        "Power Surge" 30000 { damage { out = 110 / 100 } }
    );

    let snap = math.dot_damage_snapshot(
        45,
        ActionStat::AttackPower,
        100,
        SpeedStat::SkillSpeed,
        &StatusSnapshot {
            source: &[StatusInstance {
                effect: PS,
                source: ActorId(0),
                stack: 1,
                time: 1,
            }],
            source_gauge: &[],
            target: &[],
        },
    );


    let gmm = |c, d| (snap.dot_damage(c, d, 9500))..=(snap.dot_damage(c, d, 10500));

    let n = gmm(No, No);
    let d = gmm(No, Yes);
    let c = gmm(Yes, No);
    let cd = gmm(Yes, Yes);

    eprintln!();
    eprintln!("ex nh range {:?}", n);
    eprintln!("ex dh range {:?}", d);
    eprintln!("ex ch range {:?}", c);
    eprintln!("ex cdh range {:?}", cd);
    eprintln!();

    let slice: &[u64] = include!("../incl.rs");
    for &x in slice {
        let in_expected_ranges =
            n.contains(&x) || d.contains(&x) || c.contains(&x) || cd.contains(&x);
        if !in_expected_ranges {
            eprintln!("found ??? value: {}", x);
        }
    }

    for ch in [No, Yes] {
        for dh in [No, Yes] {
            let name = match (ch, dh) {
                (No, No) => "nh",
                (No, Yes) => "dh",
                (Yes, No) => "ch",
                (Yes, Yes) => "cdh",
                _ => unreachable!(),
            };

            let ex_min = snap.dot_damage(ch, dh, 9500);
            let ex_max = snap.dot_damage(ch, dh, 10500);

            let mut real_holes = HashSet::<u64>::new();

            let mut set = HashSet::<u64>::new();

            let mut rl_min = u64::MAX;
            let mut rl_max = 0;

            for &x in slice {
                if (ex_min..=ex_max).contains(&x) {
                    set.insert(x);
                    rl_min = x.min(rl_min);
                    rl_max = x.max(rl_max);
                }
            }

            eprintln!(
                "starting {name} - ex avg {}, ex min {}, ex max {}, rl min {}, rl max {}",
                snap.dot_damage(ch, dh, 10000),
                ex_min,
                ex_max,
                rl_min,
                rl_max,
            );

            eprintln!("checking range {}..={}", rl_min, rl_max);

            let mut found_hole = false;

            println!("start real range hole check ({name}):");

            for x in ex_min..=ex_max {
                if !set.contains(&x) {
                    println!("    {x}");
                    real_holes.insert(x);
                    found_hole = true;
                }
            }

            if found_hole {
                eprintln!("found hole in range");
            }

            println!("starting expected postbuff hole check ({name}):");

            let min = snap.base * 9500 / 10000;
            let max = snap.base * 10500 / 10000;

            let run = |x: u64| x * crt_mod(&snap, ch) / 1000000 * dh_mod(&snap, dh) / 1000000;

            let mut prev = 0;

            for x in min..=max {
                if prev == 0 {
                    prev = run(x);
                } else {
                    let p = run(x);
                    for x in (prev + 1)..=(p - 1) {
                        println!(
                            "    {} {}",
                            x,
                            if !real_holes.contains(&x) { "!!!" } else { "" }
                        );
                    }
                    prev = p;
                }
            }

            eprintln!()
        }
    }
}

// copied from private methods in xivc

/// The crit multiplier based on the handling.  
/// Output is scaled by `1000000`  to allow for greater accuracy for [`CDHHandle::Avg`].
const fn crt_mod(snap: &EotSnapshot, handle: HitTypeHandle) -> u64 {
    let damage = snap.crit_damage as u64;
    let chance = snap.crit_chance as u64;

    match handle {
        // dots can never force crit/dhit but i'll keep this here
        HitTypeHandle::Force => damage * (1000 + (damage - 1000) * chance / 1000),
        HitTypeHandle::Avg => 1000000 + (damage - 1000) * chance,
        HitTypeHandle::Yes => damage * 1000,
        HitTypeHandle::No => 1000000,
    }
}

/// The direct hit multiplier based on the handling.  
/// Output is scaled by `1000000` to allow for greater accuracy for [`CDHHandle::Avg`].
const fn dh_mod(snap: &EotSnapshot, handle: HitTypeHandle) -> u64 {
    let damage = 1250;
    let chance = snap.dhit_chance as u64;

    match handle {
        HitTypeHandle::Force => damage * (1000 + (damage - 1000) * chance / 1000),
        HitTypeHandle::Avg => 1000000 + (damage - 1000) * chance,
        HitTypeHandle::Yes => damage * 1000,
        HitTypeHandle::No => 1000000,
    }
}