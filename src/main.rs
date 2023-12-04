use std::{collections::HashSet, env};

use xivc_core::{
    enums::{Clan, DamageInstance, Job},
    math::{
        ActionStat, EotSnapshot,
        HitTypeHandle::{self, *},
        PlayerInfo, PlayerStats, SpeedStat, WeaponInfo, XivMath,
    },
    status_effect,
    world::{
        status::{StatusEffect, StatusInstance, StatusSnapshot},
        ActorId,
    },
};

const PS: StatusEffect = status_effect!(
    "Power Surge" 30000 { damage { out = 110 / 100 } }
);

const TICKS: &[u64] = include!("../incl.rs.req");

fn main() {
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


    let mut args = env::args();
    // skip first arg
    args.next();

    let pre = match args.next().as_deref() {
        Some("pre") => true,
        Some("post") => false,
        _ => panic!("1st arg must be pre or post"),
    };

    let check_range = match args.next().as_deref() {
        Some("check_range") => true,
        Some("check_holes") => false,
        _ => panic!("2nd arg must be check_range or check_holes"),
    };

    if pre {
        let s = math.dot_damage_snapshot(
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
        if check_range {
            check_all_in_ex_range(s);
        } else {
            check_for_holes(s);
        }
    } else {
        let s = PostRandBuff {
            inner: math.dot_damage_snapshot(
                45,
                ActionStat::AttackPower,
                100,
                SpeedStat::SkillSpeed,
                &StatusSnapshot {
                    source: &[],
                    source_gauge: &[],
                    target: &[],
                },
            ),
            buff: StatusInstance::new(ActorId(0), PS),
        };
        if check_range {
            check_all_in_ex_range(s);
        } else {
            check_for_holes(s);
        }
    }
}

fn check_all_in_ex_range(s: impl Snapshot) {
    let gmm = |c, d| s.tick(c, d, 9500)..=s.tick(c, d, 10500);

    let n = gmm(No, No);
    let d = gmm(No, Yes);
    let c = gmm(Yes, No);
    let cd = gmm(Yes, Yes);

    eprintln!();
    eprintln!("expected nh range {:?}", n);
    eprintln!("expected dh range {:?}", d);
    eprintln!("expected ch range {:?}", c);
    eprintln!("expected cdh range {:?}", cd);
    eprintln!();
    
    let mut found_invalid = false;

    for &x in TICKS {
        let in_expected_ranges =
            n.contains(&x) || d.contains(&x) || c.contains(&x) || cd.contains(&x);
        if !in_expected_ranges {
            found_invalid = true;
            println!("found unknown value: {}", x);
        }
    }
    
    if found_invalid {
        eprintln!("found unknown values");
    }
}

fn check_for_holes(s: impl Snapshot) {
    for ch in [No, Yes] {
        for dh in [No, Yes] {
            let name = match (ch, dh) {
                (No, No) => "nh",
                (No, Yes) => "dh",
                (Yes, No) => "ch",
                (Yes, Yes) => "cdh",
                _ => unreachable!(),
            };

            let ex_min = s.tick(ch, dh, 9500);
            let ex_max = s.tick(ch, dh, 10500);

            let mut real_holes = HashSet::<u64>::new();

            let mut set = HashSet::<u64>::new();

            let mut rl_min = u64::MAX;
            let mut rl_max = 0;

            for &x in TICKS {
                if (ex_min..=ex_max).contains(&x) {
                    set.insert(x);
                    rl_min = x.min(rl_min);
                    rl_max = x.max(rl_max);
                }
            }

            eprintln!(
                "starting {name} - ex avg {}, ex min {}, ex max {}, rl min {}, rl max {}",
                s.tick(ch, dh, 10000),
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

            println!("starting expected hole check ({name}):");

            // this is just to make sure i hit every value in the range
            // i'm not sure if this is how it really works
            // but its not a problem to check *too* many values.
            let min = s.base_rand(9500);
            let max = s.base_rand(10500);

            let mut prev = 0;

            for x in min..=max {
                if prev == 0 {
                    prev = s.rand_to_dmg(x, ch, dh);
                } else {
                    let p = s.rand_to_dmg(x, ch, dh);
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

trait Snapshot {
    fn tick(&self, ch: HitTypeHandle, dh: HitTypeHandle, rand: u64) -> u64 {
        self.rand_to_dmg(self.base_rand(rand), ch, dh)
    }
    fn base_rand(&self, rand: u64) -> u64;
    fn rand_to_dmg(&self, val: u64, ch: HitTypeHandle, dh: HitTypeHandle) -> u64;
}

impl Snapshot for EotSnapshot {
    fn base_rand(&self, rand: u64) -> u64 {
        self.base * rand / 10000
    }

    fn rand_to_dmg(&self, val: u64, ch: HitTypeHandle, dh: HitTypeHandle) -> u64 {
        val * crt_mod(self, ch) / 1000000 * dh_mod(self, dh) / 1000000
    }
}

struct PostRandBuff {
    inner: EotSnapshot,
    buff: StatusInstance,
}

impl Snapshot for PostRandBuff {
    fn base_rand(&self, rand: u64) -> u64 {
        self.inner.base_rand(rand)
    }

    fn rand_to_dmg(&self, val: u64, ch: HitTypeHandle, dh: HitTypeHandle) -> u64 {
        let dmgbuff = self.buff.effect.damage.outgoing.unwrap();
        let base = self.inner.rand_to_dmg(val, ch, dh);
        dmgbuff(
            self.buff,
            DamageInstance::basic(base, ActionStat::AttackPower),
        )
        .dmg
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
