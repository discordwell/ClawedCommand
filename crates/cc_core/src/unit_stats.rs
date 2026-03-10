use crate::components::{AttackType, UnitKind};
use crate::math::Fixed;

/// Compile-time base stats for each unit type.
pub struct UnitBaseStats {
    pub health: Fixed,
    pub speed: Fixed,
    pub damage: Fixed,
    pub range: Fixed,
    pub attack_speed: u32, // ticks between attacks
    pub attack_type: AttackType,
    // Economy
    pub food_cost: u32,
    pub gpu_cost: u32,
    pub supply_cost: u32,
    pub train_time: u32, // ticks
}

/// Return the base stats for a given unit kind.
/// All values are compile-time constants — no Resource needed.
pub fn base_stats(kind: UnitKind) -> UnitBaseStats {
    match kind {
        UnitKind::Pawdler => UnitBaseStats {
            health: Fixed::from_bits(60 << 16), // 60
            speed: Fixed::from_bits(7864),      // 0.12
            damage: Fixed::from_bits(4 << 16),  // 4
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 50,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 50,
        },
        UnitKind::Nuisance => UnitBaseStats {
            health: Fixed::from_bits(80 << 16), // 80
            speed: Fixed::from_bits(11796),     // 0.18
            damage: Fixed::from_bits(8 << 16),  // 8
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 60,
        },
        UnitKind::Chonk => UnitBaseStats {
            health: Fixed::from_bits(300 << 16), // 300
            speed: Fixed::from_bits(5242),       // 0.08
            damage: Fixed::from_bits(12 << 16),  // 12
            range: Fixed::from_bits(1 << 16),    // 1
            attack_speed: 20,
            attack_type: AttackType::Melee,
            food_cost: 150,
            gpu_cost: 25,
            supply_cost: 3,
            train_time: 120,
        },
        UnitKind::FlyingFox => UnitBaseStats {
            health: Fixed::from_bits(80 << 16), // 80 (was 50)
            speed: Fixed::from_bits(14745),     // 0.225
            damage: Fixed::from_bits(10 << 16), // 10 (was 6), DPS 8.3
            range: Fixed::from_bits(2 << 16),   // 2
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 100,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Hisser => UnitBaseStats {
            health: Fixed::from_bits(70 << 16), // 70
            speed: Fixed::from_bits(7864),      // 0.12
            damage: Fixed::from_bits(12 << 16), // 12 (was 14)
            range: Fixed::from_bits(5 << 16),   // 5
            attack_speed: 13,                   // was 12
            attack_type: AttackType::Ranged,
            food_cost: 100,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Yowler => UnitBaseStats {
            health: Fixed::from_bits(90 << 16), // 90
            speed: Fixed::from_bits(9175),      // 0.14
            damage: Fixed::from_bits(5 << 16),  // 5
            range: Fixed::from_bits(4 << 16),   // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100,
            gpu_cost: 50,
            supply_cost: 2,
            train_time: 100,
        },
        UnitKind::Mouser => UnitBaseStats {
            health: Fixed::from_bits(75 << 16), // 75 (was 55)
            speed: Fixed::from_bits(13107),     // 0.20
            damage: Fixed::from_bits(10 << 16), // 10
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 25,
            supply_cost: 1,
            train_time: 60,
        },
        UnitKind::Catnapper => UnitBaseStats {
            health: Fixed::from_bits(120 << 16), // 120
            speed: Fixed::from_bits(3932),       // 0.06
            damage: Fixed::from_bits(25 << 16),  // 25
            range: Fixed::from_bits(2 << 16),    // 2
            attack_speed: 30,
            attack_type: AttackType::Ranged,
            food_cost: 200,
            gpu_cost: 50,
            supply_cost: 3,
            train_time: 150,
        },
        UnitKind::FerretSapper => UnitBaseStats {
            health: Fixed::from_bits(65 << 16), // 65
            speed: Fixed::from_bits(11141),     // 0.17
            damage: Fixed::from_bits(20 << 16), // 20
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 25,
            attack_type: AttackType::Melee,
            food_cost: 125,
            gpu_cost: 50,
            supply_cost: 2,
            train_time: 100,
        },
        UnitKind::MechCommander => UnitBaseStats {
            health: Fixed::from_bits(500 << 16), // 500
            speed: Fixed::from_bits(6553),       // 0.10
            damage: Fixed::from_bits(18 << 16),  // 18
            range: Fixed::from_bits(3 << 16),    // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400,
            gpu_cost: 200,
            supply_cost: 6,
            train_time: 250,
        },
        // --- The Murder (Corvids) ---
        UnitKind::MurderScrounger => UnitBaseStats {
            health: Fixed::from_bits(55 << 16), // 55
            speed: Fixed::from_bits(9175),      // 0.14
            damage: Fixed::from_bits(3 << 16),  // 3
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 50,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 50,
        },
        UnitKind::Sentinel => UnitBaseStats {
            health: Fixed::from_bits(60 << 16), // 60
            speed: Fixed::from_bits(10486),     // 0.16
            damage: Fixed::from_bits(12 << 16), // 12
            range: Fixed::from_bits(6 << 16),   // 6
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 75,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 60,
        },
        UnitKind::Rookclaw => UnitBaseStats {
            health: Fixed::from_bits(70 << 16), // 70
            speed: Fixed::from_bits(13107),     // 0.20
            damage: Fixed::from_bits(10 << 16), // 10
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 55,
        },
        UnitKind::Magpike => UnitBaseStats {
            health: Fixed::from_bits(55 << 16), // 55
            speed: Fixed::from_bits(11796),     // 0.18
            damage: Fixed::from_bits(8 << 16),  // 8
            range: Fixed::from_bits(4 << 16),   // 4
            attack_speed: 10,
            attack_type: AttackType::Ranged,
            food_cost: 85,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 65,
        },
        UnitKind::Magpyre => UnitBaseStats {
            health: Fixed::from_bits(50 << 16), // 50
            speed: Fixed::from_bits(11141),     // 0.17
            damage: Fixed::from_bits(8 << 16),  // 8
            range: Fixed::from_bits(3 << 16),   // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100,
            gpu_cost: 50,
            supply_cost: 2,
            train_time: 90,
        },
        UnitKind::Jaycaller => UnitBaseStats {
            health: Fixed::from_bits(85 << 16), // 85
            speed: Fixed::from_bits(9175),      // 0.14
            damage: Fixed::from_bits(5 << 16),  // 5
            range: Fixed::from_bits(4 << 16),   // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100,
            gpu_cost: 50,
            supply_cost: 2,
            train_time: 100,
        },
        UnitKind::Jayflicker => UnitBaseStats {
            health: Fixed::from_bits(60 << 16), // 60
            speed: Fixed::from_bits(10486),     // 0.16
            damage: Fixed::from_bits(7 << 16),  // 7
            range: Fixed::from_bits(3 << 16),   // 3
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 125,
            gpu_cost: 50,
            supply_cost: 2,
            train_time: 90,
        },
        UnitKind::Dusktalon => UnitBaseStats {
            health: Fixed::from_bits(65 << 16), // 65
            speed: Fixed::from_bits(13107),     // 0.20
            damage: Fixed::from_bits(15 << 16), // 15
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 125,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Hootseer => UnitBaseStats {
            health: Fixed::from_bits(100 << 16), // 100
            speed: Fixed::from_bits(6554),       // 0.10
            damage: Fixed::from_bits(8 << 16),   // 8
            range: Fixed::from_bits(5 << 16),    // 5
            attack_speed: 18,
            attack_type: AttackType::Ranged,
            food_cost: 150,
            gpu_cost: 50,
            supply_cost: 3,
            train_time: 120,
        },
        UnitKind::CorvusRex => UnitBaseStats {
            health: Fixed::from_bits(450 << 16), // 450
            speed: Fixed::from_bits(6554),       // 0.10
            damage: Fixed::from_bits(16 << 16),  // 16
            range: Fixed::from_bits(4 << 16),    // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400,
            gpu_cost: 200,
            supply_cost: 6,
            train_time: 250,
        },
        // --- Seekers of the Deep (Badgers) ---
        UnitKind::Delver => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),
            speed: Fixed::from_bits(6553),
            damage: Fixed::from_bits(3 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 18,
            attack_type: AttackType::Melee,
            food_cost: 50,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 55,
        },
        UnitKind::Ironhide => UnitBaseStats {
            health: Fixed::from_bits(250 << 16),
            speed: Fixed::from_bits(6553),
            damage: Fixed::from_bits(16 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 18,
            attack_type: AttackType::Melee,
            food_cost: 125,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 100,
        }, // speed 0.08→0.10
        UnitKind::Cragback => UnitBaseStats {
            health: Fixed::from_bits(350 << 16),
            speed: Fixed::from_bits(3932),
            damage: Fixed::from_bits(30 << 16),
            range: Fixed::from_bits(8 << 16),
            attack_speed: 30,
            attack_type: AttackType::Ranged,
            food_cost: 200,
            gpu_cost: 50,
            supply_cost: 4,
            train_time: 150,
        },
        UnitKind::Warden => UnitBaseStats {
            health: Fixed::from_bits(120 << 16), // 120 (was 150), ranged units shouldn't out-tank melee
            speed: Fixed::from_bits(6553),
            damage: Fixed::from_bits(14 << 16), // keep 14
            range: Fixed::from_bits(5 << 16),
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Sapjaw => UnitBaseStats {
            health: Fixed::from_bits(120 << 16),
            speed: Fixed::from_bits(9175),
            damage: Fixed::from_bits(14 << 16), // 14 (was 18), DPS 1.17. Tanky identity via HP, not DPS
            range: Fixed::from_bits(1 << 16),
            attack_speed: 12,
            attack_type: AttackType::Melee,
            food_cost: 100,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Wardenmother => UnitBaseStats {
            health: Fixed::from_bits(600 << 16),
            speed: Fixed::from_bits(5242),
            damage: Fixed::from_bits(22 << 16),
            range: Fixed::from_bits(3 << 16),
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 450,
            gpu_cost: 250,
            supply_cost: 6,
            train_time: 280,
        },
        UnitKind::SeekerTunneler => UnitBaseStats {
            health: Fixed::from_bits(80 << 16),
            speed: Fixed::from_bits(9175),
            damage: Fixed::from_bits(6 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 20,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 25,
            supply_cost: 1,
            train_time: 70,
        },
        UnitKind::Embermaw => UnitBaseStats {
            health: Fixed::from_bits(90 << 16),
            speed: Fixed::from_bits(6553),
            damage: Fixed::from_bits(16 << 16),
            range: Fixed::from_bits(6 << 16),
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 125,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 90,
        },
        UnitKind::Dustclaw => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),
            speed: Fixed::from_bits(10486),
            damage: Fixed::from_bits(10 << 16), // 10 (was 12), DPS 1.0. Fast harasser, not a brawler
            range: Fixed::from_bits(1 << 16),
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 60,
        },
        UnitKind::Gutripper => UnitBaseStats {
            health: Fixed::from_bits(160 << 16),
            speed: Fixed::from_bits(7864),
            damage: Fixed::from_bits(20 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 150,
            gpu_cost: 25,
            supply_cost: 3,
            train_time: 120,
        },
        // --- Croak (Axolotls) ---
        UnitKind::Ponderer => UnitBaseStats {
            health: Fixed::from_bits(55 << 16), // 55
            speed: Fixed::from_bits(6553),      // 0.10
            damage: Fixed::from_bits(3 << 16),  // 3
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 18,
            attack_type: AttackType::Melee,
            food_cost: 50,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 50,
        },
        UnitKind::Regeneron => UnitBaseStats {
            health: Fixed::from_bits(80 << 16), // 80
            speed: Fixed::from_bits(10485),     // 0.16
            damage: Fixed::from_bits(8 << 16),  // 8
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 70,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 55,
        },
        UnitKind::Broodmother => UnitBaseStats {
            health: Fixed::from_bits(100 << 16), // 100
            speed: Fixed::from_bits(6553),       // 0.10
            damage: Fixed::from_bits(4 << 16),   // 4
            range: Fixed::from_bits(3 << 16),    // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 125,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 100,
        },
        UnitKind::Gulper => UnitBaseStats {
            health: Fixed::from_bits(300 << 16), // 300
            speed: Fixed::from_bits(4587),       // 0.07
            damage: Fixed::from_bits(14 << 16),  // 14
            range: Fixed::from_bits(1 << 16),    // 1
            attack_speed: 16,
            attack_type: AttackType::Melee,
            food_cost: 160,
            gpu_cost: 25,
            supply_cost: 3,
            train_time: 115,
        },
        UnitKind::Eftsaber => UnitBaseStats {
            health: Fixed::from_bits(60 << 16), // 60
            speed: Fixed::from_bits(11796),     // 0.18
            damage: Fixed::from_bits(12 << 16), // 12
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 9,
            attack_type: AttackType::Melee,
            food_cost: 100,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Croaker => UnitBaseStats {
            health: Fixed::from_bits(65 << 16), // 65
            speed: Fixed::from_bits(6553),      // 0.10
            damage: Fixed::from_bits(16 << 16), // 16
            range: Fixed::from_bits(6 << 16),   // 6
            attack_speed: 18,
            attack_type: AttackType::Ranged,
            food_cost: 90,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 70,
        },
        UnitKind::Leapfrog => UnitBaseStats {
            health: Fixed::from_bits(70 << 16), // 70
            speed: Fixed::from_bits(11141),     // 0.17
            damage: Fixed::from_bits(8 << 16),  // 8
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 60,
        },
        UnitKind::Shellwarden => UnitBaseStats {
            health: Fixed::from_bits(350 << 16), // 350
            speed: Fixed::from_bits(3932),       // 0.06
            damage: Fixed::from_bits(6 << 16),   // 6
            range: Fixed::from_bits(1 << 16),    // 1
            attack_speed: 22,
            attack_type: AttackType::Melee,
            food_cost: 175,
            gpu_cost: 50,
            supply_cost: 4,
            train_time: 140,
        },
        UnitKind::Bogwhisper => UnitBaseStats {
            health: Fixed::from_bits(80 << 16), // 80
            speed: Fixed::from_bits(7209),      // 0.11
            damage: Fixed::from_bits(5 << 16),  // 5
            range: Fixed::from_bits(5 << 16),   // 5
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 125,
            gpu_cost: 50,
            supply_cost: 3,
            train_time: 100,
        },
        // --- The Clawed (Mice) ---
        UnitKind::Nibblet => UnitBaseStats {
            health: Fixed::from_bits(40 << 16), // 40
            speed: Fixed::from_bits(9830),      // 0.15
            damage: Fixed::from_bits(3 << 16),  // 3
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 30,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 35,
        },
        UnitKind::Swarmer => UnitBaseStats {
            health: Fixed::from_bits(70 << 16), // 70 (was 45)
            speed: Fixed::from_bits(13107),     // 0.20 (was 0.15)
            damage: Fixed::from_bits(7 << 16),  // 7 (was 5)
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 6,                    // was 8 (DPS 1.17)
            attack_type: AttackType::Melee,
            food_cost: 40,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 30,
        },
        UnitKind::Gnawer => UnitBaseStats {
            health: Fixed::from_bits(55 << 16), // 55
            speed: Fixed::from_bits(6553),      // 0.10
            damage: Fixed::from_bits(6 << 16),  // 6
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 12,
            attack_type: AttackType::Melee,
            food_cost: 50,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 45,
        },
        UnitKind::Shrieker => UnitBaseStats {
            health: Fixed::from_bits(45 << 16), // 45
            speed: Fixed::from_bits(9175),      // 0.14
            damage: Fixed::from_bits(8 << 16),  // 8
            range: Fixed::from_bits(3 << 16),   // 3
            attack_speed: 10,
            attack_type: AttackType::Ranged,
            food_cost: 55,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 40,
        },
        UnitKind::Tunneler => UnitBaseStats {
            health: Fixed::from_bits(60 << 16), // 60
            speed: Fixed::from_bits(5898),      // 0.09
            damage: Fixed::from_bits(4 << 16),  // 4
            range: Fixed::from_bits(1 << 16),   // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 75,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 70,
        },
        UnitKind::Sparks => UnitBaseStats {
            health: Fixed::from_bits(50 << 16), // 50 (was 40)
            speed: Fixed::from_bits(11141),     // 0.17
            damage: Fixed::from_bits(10 << 16), // 10 (was 7)
            range: Fixed::from_bits(4 << 16),   // 4 (was 2)
            attack_speed: 10,                   // was 12 (DPS 1.0)
            attack_type: AttackType::Ranged,
            food_cost: 60,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 50, // gpu 15→0
        },
        UnitKind::Quillback => UnitBaseStats {
            health: Fixed::from_bits(200 << 16), // 200
            speed: Fixed::from_bits(3932),       // 0.06
            damage: Fixed::from_bits(10 << 16),  // 10
            range: Fixed::from_bits(1 << 16),    // 1
            attack_speed: 18,
            attack_type: AttackType::Melee,
            food_cost: 100,
            gpu_cost: 15,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::Whiskerwitch => UnitBaseStats {
            health: Fixed::from_bits(50 << 16), // 50
            speed: Fixed::from_bits(7864),      // 0.12
            damage: Fixed::from_bits(4 << 16),  // 4
            range: Fixed::from_bits(4 << 16),   // 4
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 70,
            gpu_cost: 30,
            supply_cost: 2,
            train_time: 65,
        },
        UnitKind::Plaguetail => UnitBaseStats {
            health: Fixed::from_bits(60 << 16), // 60
            speed: Fixed::from_bits(7209),      // 0.11
            damage: Fixed::from_bits(10 << 16), // 10 (was 6)
            range: Fixed::from_bits(4 << 16),   // 4 (was 2)
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 45,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 40,
        },
        UnitKind::WarrenMarshal => UnitBaseStats {
            health: Fixed::from_bits(300 << 16), // 300
            speed: Fixed::from_bits(5242),       // 0.08
            damage: Fixed::from_bits(12 << 16),  // 12
            range: Fixed::from_bits(3 << 16),    // 3
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 250,
            gpu_cost: 125,
            supply_cost: 4,
            train_time: 200,
        },
        UnitKind::MurkCommander => UnitBaseStats {
            health: Fixed::from_bits(450 << 16), // 450
            speed: Fixed::from_bits(5898),       // 0.09
            damage: Fixed::from_bits(15 << 16),  // 15
            range: Fixed::from_bits(3 << 16),    // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400,
            gpu_cost: 200,
            supply_cost: 6,
            train_time: 250,
        },
        // --- LLAMA (Raccoons) ---
        UnitKind::Scrounger => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),
            speed: Fixed::from_bits(7209),
            damage: Fixed::from_bits(3 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 45,
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 45,
        },
        UnitKind::Bandit => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),
            speed: Fixed::from_bits(12452),
            damage: Fixed::from_bits(8 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 70, // 70 (was 65)
            gpu_cost: 0,
            supply_cost: 1,
            train_time: 55,
        },
        UnitKind::HeapTitan => UnitBaseStats {
            health: Fixed::from_bits(280 << 16),
            speed: Fixed::from_bits(4588),
            damage: Fixed::from_bits(10 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 22,
            attack_type: AttackType::Melee,
            food_cost: 140,
            gpu_cost: 20,
            supply_cost: 3,
            train_time: 110,
        },
        UnitKind::GlitchRat => UnitBaseStats {
            health: Fixed::from_bits(40 << 16),
            speed: Fixed::from_bits(14418),
            damage: Fixed::from_bits(5 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 12,
            attack_type: AttackType::Melee,
            food_cost: 60,
            gpu_cost: 15,
            supply_cost: 1,
            train_time: 50,
        },
        UnitKind::PatchPossum => UnitBaseStats {
            health: Fixed::from_bits(80 << 16),
            speed: Fixed::from_bits(8520),
            damage: Fixed::from_bits(4 << 16),
            range: Fixed::from_bits(3 << 16),
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 90,
            gpu_cost: 25,
            supply_cost: 2,
            train_time: 80,
        },
        UnitKind::GreaseMonkey => UnitBaseStats {
            health: Fixed::from_bits(65 << 16),
            speed: Fixed::from_bits(8520),
            damage: Fixed::from_bits(12 << 16),
            range: Fixed::from_bits(5 << 16),
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 90,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 75, // speed 0.10→0.13, range 4→5
        },
        UnitKind::DeadDropUnit => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),
            speed: Fixed::from_bits(9175),
            damage: Fixed::from_bits(8 << 16),
            range: Fixed::from_bits(1 << 16),
            attack_speed: 12,
            attack_type: AttackType::Melee,
            food_cost: 80,
            gpu_cost: 20,
            supply_cost: 1,
            train_time: 65,
        },
        UnitKind::Wrecker => UnitBaseStats {
            health: Fixed::from_bits(100 << 16),
            speed: Fixed::from_bits(7864),
            damage: Fixed::from_bits(14 << 16), // 14
            range: Fixed::from_bits(1 << 16),
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 95,
            gpu_cost: 0,
            supply_cost: 2,
            train_time: 85, // food 110→95
        },
        UnitKind::DumpsterDiver => UnitBaseStats {
            health: Fixed::from_bits(75 << 16),
            speed: Fixed::from_bits(7209),
            damage: Fixed::from_bits(6 << 16),
            range: Fixed::from_bits(2 << 16),
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 85,
            gpu_cost: 20,
            supply_cost: 2,
            train_time: 70,
        },
        UnitKind::JunkyardKing => UnitBaseStats {
            health: Fixed::from_bits(450 << 16),
            speed: Fixed::from_bits(5898),
            damage: Fixed::from_bits(16 << 16),
            range: Fixed::from_bits(3 << 16),
            attack_speed: 16,
            attack_type: AttackType::Ranged,
            food_cost: 375,
            gpu_cost: 175,
            supply_cost: 6,
            train_time: 230,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_have_stats() {
        let kinds = [
            UnitKind::Pawdler,
            UnitKind::Nuisance,
            UnitKind::Chonk,
            UnitKind::FlyingFox,
            UnitKind::Hisser,
            UnitKind::Yowler,
            UnitKind::Mouser,
            UnitKind::Catnapper,
            UnitKind::FerretSapper,
            UnitKind::MechCommander,
            // The Clawed (Mice)
            UnitKind::Nibblet,
            UnitKind::Swarmer,
            UnitKind::Gnawer,
            UnitKind::Shrieker,
            UnitKind::Tunneler,
            UnitKind::Sparks,
            UnitKind::Quillback,
            UnitKind::Whiskerwitch,
            UnitKind::Plaguetail,
            UnitKind::WarrenMarshal,
            // The Murder (Corvids)
            UnitKind::MurderScrounger,
            UnitKind::Sentinel,
            UnitKind::Rookclaw,
            UnitKind::Magpike,
            UnitKind::Magpyre,
            UnitKind::Jaycaller,
            UnitKind::Jayflicker,
            UnitKind::Dusktalon,
            UnitKind::Hootseer,
            UnitKind::CorvusRex,
            // Croak (Axolotls)
            UnitKind::Ponderer,
            UnitKind::Regeneron,
            UnitKind::Broodmother,
            UnitKind::Gulper,
            UnitKind::Eftsaber,
            UnitKind::Croaker,
            UnitKind::Leapfrog,
            UnitKind::Shellwarden,
            UnitKind::Bogwhisper,
            UnitKind::MurkCommander,
            // LLAMA (Raccoons)
            UnitKind::Scrounger,
            UnitKind::Bandit,
            UnitKind::HeapTitan,
            UnitKind::GlitchRat,
            UnitKind::PatchPossum,
            UnitKind::GreaseMonkey,
            UnitKind::DeadDropUnit,
            UnitKind::Wrecker,
            UnitKind::DumpsterDiver,
            UnitKind::JunkyardKing,
            // Seekers of the Deep (Badgers)
            UnitKind::Delver,
            UnitKind::Ironhide,
            UnitKind::Cragback,
            UnitKind::Warden,
            UnitKind::Sapjaw,
            UnitKind::Wardenmother,
            UnitKind::SeekerTunneler,
            UnitKind::Embermaw,
            UnitKind::Dustclaw,
            UnitKind::Gutripper,
        ];
        for kind in kinds {
            let stats = base_stats(kind);
            assert!(
                stats.health > Fixed::ZERO,
                "{kind:?} should have positive health"
            );
            assert!(
                stats.speed > Fixed::ZERO,
                "{kind:?} should have positive speed"
            );
            assert!(
                stats.damage > Fixed::ZERO,
                "{kind:?} should have positive damage"
            );
            assert!(
                stats.range > Fixed::ZERO,
                "{kind:?} should have positive range"
            );
            assert!(
                stats.attack_speed > 0,
                "{kind:?} should have positive attack_speed"
            );
        }
    }

    #[test]
    fn melee_units_have_range_one() {
        let melee_kinds = [
            UnitKind::Pawdler,
            UnitKind::Nuisance,
            UnitKind::Chonk,
            UnitKind::Mouser,
            UnitKind::FerretSapper,
            // Clawed melee
            UnitKind::Nibblet,
            UnitKind::Swarmer,
            UnitKind::Gnawer,
            UnitKind::Tunneler,
            UnitKind::Quillback,
            // Murder melee
            UnitKind::MurderScrounger,
            UnitKind::Rookclaw,
            UnitKind::Dusktalon,
            // Croak melee
            UnitKind::Ponderer,
            UnitKind::Regeneron,
            UnitKind::Gulper,
            UnitKind::Eftsaber,
            UnitKind::Leapfrog,
            UnitKind::Shellwarden,
            // LLAMA melee
            UnitKind::Scrounger,
            UnitKind::Bandit,
            UnitKind::HeapTitan,
            UnitKind::GlitchRat,
            UnitKind::DeadDropUnit,
            UnitKind::Wrecker,
            // Seekers melee
            UnitKind::Delver,
            UnitKind::Ironhide,
            UnitKind::Sapjaw,
            UnitKind::SeekerTunneler,
            UnitKind::Dustclaw,
            UnitKind::Gutripper,
        ];
        for kind in melee_kinds {
            let stats = base_stats(kind);
            assert_eq!(
                stats.attack_type,
                AttackType::Melee,
                "{kind:?} should be melee"
            );
            assert_eq!(
                stats.range,
                Fixed::from_bits(1 << 16),
                "{kind:?} melee should have range 1"
            );
        }
    }

    #[test]
    fn ranged_units_have_range_greater_than_one() {
        let ranged_kinds = [
            UnitKind::FlyingFox,
            UnitKind::Hisser,
            UnitKind::Yowler,
            UnitKind::Catnapper,
            UnitKind::MechCommander,
            // Clawed ranged
            UnitKind::Shrieker,
            UnitKind::Sparks,
            UnitKind::Whiskerwitch,
            UnitKind::Plaguetail,
            UnitKind::WarrenMarshal,
            // Murder ranged
            UnitKind::Sentinel,
            UnitKind::Magpike,
            UnitKind::Magpyre,
            UnitKind::Jaycaller,
            UnitKind::Jayflicker,
            UnitKind::Hootseer,
            UnitKind::CorvusRex,
            // Croak ranged
            UnitKind::Broodmother,
            UnitKind::Croaker,
            UnitKind::Bogwhisper,
            UnitKind::MurkCommander,
            // LLAMA ranged
            UnitKind::PatchPossum,
            UnitKind::GreaseMonkey,
            UnitKind::DumpsterDiver,
            UnitKind::JunkyardKing,
            // Seekers ranged
            UnitKind::Cragback,
            UnitKind::Warden,
            UnitKind::Wardenmother,
            UnitKind::Embermaw,
        ];
        for kind in ranged_kinds {
            let stats = base_stats(kind);
            assert_eq!(
                stats.attack_type,
                AttackType::Ranged,
                "{kind:?} should be ranged"
            );
            assert!(
                stats.range > Fixed::from_bits(1 << 16),
                "{kind:?} ranged should have range > 1"
            );
        }
    }

    #[test]
    fn chonk_is_tankiest() {
        // Among non-hero units, Chonk should have the most HP
        let chonk = base_stats(UnitKind::Chonk);
        let nuisance = base_stats(UnitKind::Nuisance);
        let hisser = base_stats(UnitKind::Hisser);
        assert!(chonk.health > nuisance.health);
        assert!(chonk.health > hisser.health);
    }

    #[test]
    fn mech_commander_is_strongest() {
        let mech = base_stats(UnitKind::MechCommander);
        let chonk = base_stats(UnitKind::Chonk);
        assert!(mech.health > chonk.health);
    }

    #[test]
    fn shellwarden_tankiest_croak() {
        let shellwarden = base_stats(UnitKind::Shellwarden);
        let gulper = base_stats(UnitKind::Gulper);
        let regeneron = base_stats(UnitKind::Regeneron);
        assert!(shellwarden.health > gulper.health);
        assert!(shellwarden.health > regeneron.health);
    }

    #[test]
    fn murk_commander_strongest_croak() {
        let murk = base_stats(UnitKind::MurkCommander);
        let shellwarden = base_stats(UnitKind::Shellwarden);
        assert!(murk.health > shellwarden.health);
    }

    #[test]
    fn swarmer_is_cheapest_clawed_combat() {
        let swarmer = base_stats(UnitKind::Swarmer);
        let gnawer = base_stats(UnitKind::Gnawer);
        let shrieker = base_stats(UnitKind::Shrieker);
        assert!(swarmer.food_cost < gnawer.food_cost);
        assert!(swarmer.food_cost < shrieker.food_cost);
    }

    #[test]
    fn quillback_is_clawed_tankiest_non_hero() {
        let quillback = base_stats(UnitKind::Quillback);
        let swarmer = base_stats(UnitKind::Swarmer);
        let gnawer = base_stats(UnitKind::Gnawer);
        assert!(quillback.health > swarmer.health);
        assert!(quillback.health > gnawer.health);
    }

    #[test]
    fn warren_marshal_is_clawed_hero() {
        let marshal = base_stats(UnitKind::WarrenMarshal);
        let quillback = base_stats(UnitKind::Quillback);
        assert!(marshal.health > quillback.health);
    }

    // --- Murder unit stat tests ---

    #[test]
    fn all_murder_kinds_have_stats() {
        let kinds = [
            UnitKind::MurderScrounger,
            UnitKind::Sentinel,
            UnitKind::Rookclaw,
            UnitKind::Magpike,
            UnitKind::Magpyre,
            UnitKind::Jaycaller,
            UnitKind::Jayflicker,
            UnitKind::Dusktalon,
            UnitKind::Hootseer,
            UnitKind::CorvusRex,
        ];
        for kind in kinds {
            let stats = base_stats(kind);
            assert!(
                stats.health > Fixed::ZERO,
                "{kind:?} should have positive health"
            );
            assert!(
                stats.speed > Fixed::ZERO,
                "{kind:?} should have positive speed"
            );
        }
    }

    #[test]
    fn corvus_rex_is_murder_strongest() {
        let rex = base_stats(UnitKind::CorvusRex);
        let hootseer = base_stats(UnitKind::Hootseer);
        let rookclaw = base_stats(UnitKind::Rookclaw);
        assert!(rex.health > hootseer.health);
        assert!(rex.health > rookclaw.health);
    }

    #[test]
    fn murder_melee_units_have_range_one() {
        let melee_kinds = [
            UnitKind::MurderScrounger,
            UnitKind::Rookclaw,
            UnitKind::Dusktalon,
        ];
        for kind in melee_kinds {
            let stats = base_stats(kind);
            assert_eq!(
                stats.attack_type,
                AttackType::Melee,
                "{kind:?} should be melee"
            );
            assert_eq!(
                stats.range,
                Fixed::from_bits(1 << 16),
                "{kind:?} melee should have range 1"
            );
        }
    }

    #[test]
    fn murder_ranged_units_have_range_gt_one() {
        let ranged_kinds = [
            UnitKind::Sentinel,
            UnitKind::Magpike,
            UnitKind::Magpyre,
            UnitKind::Jaycaller,
            UnitKind::Jayflicker,
            UnitKind::Hootseer,
            UnitKind::CorvusRex,
        ];
        for kind in ranged_kinds {
            let stats = base_stats(kind);
            assert_eq!(
                stats.attack_type,
                AttackType::Ranged,
                "{kind:?} should be ranged"
            );
            assert!(
                stats.range > Fixed::from_bits(1 << 16),
                "{kind:?} ranged should have range > 1"
            );
        }
    }

    // --- Seekers of the Deep tests ---

    #[test]
    fn all_seekers_have_stats() {
        let kinds = [
            UnitKind::Delver,
            UnitKind::Ironhide,
            UnitKind::Cragback,
            UnitKind::Warden,
            UnitKind::Sapjaw,
            UnitKind::Wardenmother,
            UnitKind::SeekerTunneler,
            UnitKind::Embermaw,
            UnitKind::Dustclaw,
            UnitKind::Gutripper,
        ];
        for kind in kinds {
            let stats = base_stats(kind);
            assert!(
                stats.health > Fixed::ZERO,
                "{kind:?} should have positive health"
            );
            assert!(
                stats.speed > Fixed::ZERO,
                "{kind:?} should have positive speed"
            );
            assert!(
                stats.damage > Fixed::ZERO,
                "{kind:?} should have positive damage"
            );
            assert!(
                stats.range > Fixed::ZERO,
                "{kind:?} should have positive range"
            );
            assert!(
                stats.attack_speed > 0,
                "{kind:?} should have positive attack_speed"
            );
        }
    }

    #[test]
    fn seekers_melee_units_have_range_one() {
        let melee_kinds = [
            UnitKind::Delver,
            UnitKind::Ironhide,
            UnitKind::Sapjaw,
            UnitKind::SeekerTunneler,
            UnitKind::Dustclaw,
            UnitKind::Gutripper,
        ];
        for kind in melee_kinds {
            let stats = base_stats(kind);
            assert_eq!(
                stats.attack_type,
                AttackType::Melee,
                "{kind:?} should be melee"
            );
            assert_eq!(
                stats.range,
                Fixed::from_bits(1 << 16),
                "{kind:?} melee should have range 1"
            );
        }
    }

    #[test]
    fn seekers_ranged_units_have_range_gt_one() {
        let ranged_kinds = [
            UnitKind::Cragback,
            UnitKind::Warden,
            UnitKind::Wardenmother,
            UnitKind::Embermaw,
        ];
        for kind in ranged_kinds {
            let stats = base_stats(kind);
            assert_eq!(
                stats.attack_type,
                AttackType::Ranged,
                "{kind:?} should be ranged"
            );
            assert!(
                stats.range > Fixed::from_bits(1 << 16),
                "{kind:?} ranged should have range > 1"
            );
        }
    }

    #[test]
    fn wardenmother_tankiest_seekers() {
        let wm = base_stats(UnitKind::Wardenmother);
        let ironhide = base_stats(UnitKind::Ironhide);
        let cragback = base_stats(UnitKind::Cragback);
        assert!(wm.health > ironhide.health);
        assert!(wm.health > cragback.health);
    }

    #[test]
    fn wardenmother_most_expensive_seekers() {
        let wm = base_stats(UnitKind::Wardenmother);
        assert_eq!(wm.food_cost, 450);
        assert_eq!(wm.gpu_cost, 250);
        assert_eq!(wm.supply_cost, 6);
    }

    // --- LLAMA unit tests ---

    #[test]
    fn heap_titan_tankiest_llama() {
        let ht = base_stats(UnitKind::HeapTitan);
        let bandit = base_stats(UnitKind::Bandit);
        let wrecker = base_stats(UnitKind::Wrecker);
        assert!(ht.health > bandit.health);
        assert!(ht.health > wrecker.health);
    }

    #[test]
    fn junkyard_king_strongest_llama() {
        let jk = base_stats(UnitKind::JunkyardKing);
        let ht = base_stats(UnitKind::HeapTitan);
        assert!(jk.health > ht.health);
    }
}

/// Cross-faction balance regression tests.
/// Pure computation — no Bevy World needed. Catches stat imbalances that
/// previously required full 9000-tick integration games to discover.
#[cfg(test)]
mod balance_tests {
    use super::*;
    use crate::tuning::*;
    use crate::upgrade_stats::upgrade_stats;

    /// All non-worker combat units across every faction.
    const ALL_COMBAT_UNITS: &[UnitKind] = &[
        // catGPT
        UnitKind::Nuisance,
        UnitKind::Chonk,
        UnitKind::FlyingFox,
        UnitKind::Hisser,
        UnitKind::Yowler,
        UnitKind::Mouser,
        UnitKind::Catnapper,
        UnitKind::FerretSapper,
        UnitKind::MechCommander,
        // The Murder
        UnitKind::Sentinel,
        UnitKind::Rookclaw,
        UnitKind::Magpike,
        UnitKind::Magpyre,
        UnitKind::Jaycaller,
        UnitKind::Jayflicker,
        UnitKind::Dusktalon,
        UnitKind::Hootseer,
        UnitKind::CorvusRex,
        // Seekers of the Deep
        UnitKind::Ironhide,
        UnitKind::Cragback,
        UnitKind::Warden,
        UnitKind::Sapjaw,
        UnitKind::Wardenmother,
        UnitKind::SeekerTunneler,
        UnitKind::Embermaw,
        UnitKind::Dustclaw,
        UnitKind::Gutripper,
        // The Clawed
        UnitKind::Swarmer,
        UnitKind::Gnawer,
        UnitKind::Shrieker,
        UnitKind::Tunneler,
        UnitKind::Sparks,
        UnitKind::Quillback,
        UnitKind::Whiskerwitch,
        UnitKind::Plaguetail,
        UnitKind::WarrenMarshal,
        // Croak
        UnitKind::Regeneron,
        UnitKind::Broodmother,
        UnitKind::Gulper,
        UnitKind::Eftsaber,
        UnitKind::Croaker,
        UnitKind::Leapfrog,
        UnitKind::Shellwarden,
        UnitKind::Bogwhisper,
        UnitKind::MurkCommander,
        // LLAMA
        UnitKind::Bandit,
        UnitKind::HeapTitan,
        UnitKind::GlitchRat,
        UnitKind::PatchPossum,
        UnitKind::GreaseMonkey,
        UnitKind::DeadDropUnit,
        UnitKind::Wrecker,
        UnitKind::DumpsterDiver,
        UnitKind::JunkyardKing,
    ];

    fn dps(s: &UnitBaseStats) -> f64 {
        s.damage.to_num::<f64>() / s.attack_speed as f64
    }

    /// Test 1: No combat unit's combat efficiency (DPS * HP / food) should exceed
    /// 4.0x the cross-faction median. Uses the DPS*HP product to account for the
    /// intentional HP/DPS tradeoff between swarm units and elite units.
    /// The 4.0x bound allows natural variance between tank/glass-cannon designs
    /// while catching truly broken outliers.
    #[test]
    fn no_unit_combat_efficiency_exceeds_bound() {
        let mut ratios: Vec<(UnitKind, f64)> = ALL_COMBAT_UNITS
            .iter()
            .filter_map(|&kind| {
                let s = base_stats(kind);
                if s.food_cost == 0 {
                    return None;
                }
                let efficiency = dps(&s) * s.health.to_num::<f64>() / s.food_cost as f64;
                Some((kind, efficiency))
            })
            .collect();

        ratios.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let median = if ratios.len() % 2 == 0 {
            (ratios[ratios.len() / 2 - 1].1 + ratios[ratios.len() / 2].1) / 2.0
        } else {
            ratios[ratios.len() / 2].1
        };

        let bound = 4.0 * median;
        for &(kind, ratio) in &ratios {
            assert!(
                ratio <= bound,
                "{kind:?} combat efficiency (DPS*HP/food) {ratio:.3} exceeds 4.0x median ({median:.3}, bound={bound:.3})"
            );
        }
    }

    /// Test 2: Ranged units should trade HP for range.
    /// Average HP/food for ranged units must be less than average HP/food for melee units.
    /// Catches issues like Warden having 150 HP (more than most melee tanks at similar cost).
    #[test]
    fn ranged_units_trade_hp_for_range() {
        let mut ranged_hp_food = Vec::new();
        let mut melee_hp_food = Vec::new();

        for &kind in ALL_COMBAT_UNITS {
            let s = base_stats(kind);
            if s.food_cost == 0 {
                continue;
            }
            let ratio = s.health.to_num::<f64>() / s.food_cost as f64;
            match s.attack_type {
                AttackType::Ranged => ranged_hp_food.push(ratio),
                AttackType::Melee => melee_hp_food.push(ratio),
            }
        }

        let ranged_avg: f64 = ranged_hp_food.iter().sum::<f64>() / ranged_hp_food.len() as f64;
        let melee_avg: f64 = melee_hp_food.iter().sum::<f64>() / melee_hp_food.len() as f64;

        assert!(
            ranged_avg < melee_avg,
            "Ranged avg HP/food ({ranged_avg:.3}) should be less than melee avg ({melee_avg:.3})"
        );
    }

    /// Test 3: All non-worker combat units must cost at least 40 food.
    /// Prevents accidentally creating ultra-cheap spam units.
    /// (The Clawed's Swarmer at 40f is the intentional floor for swarm-identity factions.)
    #[test]
    fn combat_unit_minimum_food_cost() {
        const MIN_FOOD: u32 = 40;
        for &kind in ALL_COMBAT_UNITS {
            let s = base_stats(kind);
            assert!(
                s.food_cost >= MIN_FOOD,
                "{kind:?} food_cost {} is below minimum {MIN_FOOD}",
                s.food_cost
            );
        }
    }

    /// Test 6: Each faction's defense tower DPS >= 1.5x the weakest unit DPS in that
    /// faction's preferred army. Towers must justify their food/gpu investment.
    #[test]
    fn tower_dps_worth_at_least_one_point_five_units() {
        // (faction_name, tower_building, tower_damage_const, tower_attack_speed, unit_preferences)
        struct FactionTowerCheck {
            name: &'static str,
            tower_damage: Fixed,
            tower_attack_speed: u32,
            preferred_units: &'static [UnitKind],
        }

        let checks = [
            FactionTowerCheck {
                name: "CatGPT",
                tower_damage: TOWER_DAMAGE_LASER_POINTER,
                tower_attack_speed: TOWER_ATTACK_SPEED_LASER_POINTER,
                preferred_units: &[
                    UnitKind::Nuisance,
                    UnitKind::Hisser,
                    UnitKind::Chonk,
                    UnitKind::FlyingFox,
                ],
            },
            FactionTowerCheck {
                name: "TheClawed",
                tower_damage: TOWER_DAMAGE_SQUEAK_TOWER,
                tower_attack_speed: TOWER_ATTACK_SPEED_SQUEAK_TOWER,
                preferred_units: &[
                    UnitKind::Swarmer,
                    UnitKind::Plaguetail,
                    UnitKind::Gnawer,
                    UnitKind::Sparks,
                ],
            },
            FactionTowerCheck {
                name: "Seekers",
                tower_damage: TOWER_DAMAGE_SLAG_THROWER,
                tower_attack_speed: TOWER_ATTACK_SPEED_SLAG_THROWER,
                preferred_units: &[
                    UnitKind::Sapjaw,
                    UnitKind::Dustclaw,
                    UnitKind::Warden,
                    UnitKind::Ironhide,
                ],
            },
            FactionTowerCheck {
                name: "Murder",
                tower_damage: TOWER_DAMAGE_WATCHTOWER,
                tower_attack_speed: TOWER_ATTACK_SPEED_WATCHTOWER,
                preferred_units: &[
                    UnitKind::Rookclaw,
                    UnitKind::Sentinel,
                    UnitKind::Magpike,
                    UnitKind::Jaycaller,
                ],
            },
            FactionTowerCheck {
                name: "LLAMA",
                tower_damage: TOWER_DAMAGE_TETANUS_TOWER,
                tower_attack_speed: TOWER_ATTACK_SPEED_TETANUS_TOWER,
                preferred_units: &[UnitKind::Bandit, UnitKind::GreaseMonkey, UnitKind::Wrecker],
            },
            FactionTowerCheck {
                name: "Croak",
                tower_damage: TOWER_DAMAGE_SPORE_TOWER,
                tower_attack_speed: TOWER_ATTACK_SPEED_SPORE_TOWER,
                preferred_units: &[
                    UnitKind::Regeneron,
                    UnitKind::Croaker,
                    UnitKind::Leapfrog,
                    UnitKind::Gulper,
                    UnitKind::Shellwarden,
                    UnitKind::Broodmother,
                ],
            },
        ];

        for check in &checks {
            let tower_dps = check.tower_damage.to_num::<f64>() / check.tower_attack_speed as f64;

            let min_unit_dps = check
                .preferred_units
                .iter()
                .map(|&kind| dps(&base_stats(kind)))
                .fold(f64::INFINITY, f64::min);

            let bound = 1.5 * min_unit_dps;
            assert!(
                tower_dps >= bound,
                "{} tower DPS {tower_dps:.2} < 1.5x weakest unit DPS ({min_unit_dps:.3}, bound={bound:.3})",
                check.name
            );
        }
    }

    /// Test 7: All damage upgrades cost the same, all HP upgrades cost the same,
    /// all speed upgrades cost the same. Prevents accidentally making one faction's
    /// upgrades cheaper than another's.
    #[test]
    fn upgrade_costs_consistent_across_factions() {
        use crate::components::UpgradeType;

        let damage_upgrades = [
            UpgradeType::SharperClaws,
            UpgradeType::SharperTeeth,
            UpgradeType::SharperFangs,
            UpgradeType::SharperTalons,
            UpgradeType::RustyFangs,
            UpgradeType::SlickerMucus,
        ];
        let hp_upgrades = [
            UpgradeType::ThickerFur,
            UpgradeType::ThickerHide,
            UpgradeType::ReinforcedHide,
            UpgradeType::HardenedPlumage,
            UpgradeType::ScrapPlating,
            UpgradeType::TougherHide,
        ];
        let speed_upgrades = [
            UpgradeType::NimblePaws,
            UpgradeType::QuickPaws,
            UpgradeType::SteadyStance,
            UpgradeType::SwiftWings,
            UpgradeType::TrashRunning,
            UpgradeType::AmphibianAgility,
        ];

        fn assert_same_cost(category: &str, upgrades: &[UpgradeType]) {
            let first = upgrade_stats(upgrades[0]);
            for &u in &upgrades[1..] {
                let s = upgrade_stats(u);
                assert_eq!(
                    s.research_time, first.research_time,
                    "{category} upgrade {u:?} research_time {} != {} (from {:?})",
                    s.research_time, first.research_time, upgrades[0]
                );
                assert_eq!(
                    s.food_cost, first.food_cost,
                    "{category} upgrade {u:?} food_cost {} != {} (from {:?})",
                    s.food_cost, first.food_cost, upgrades[0]
                );
                assert_eq!(
                    s.gpu_cost, first.gpu_cost,
                    "{category} upgrade {u:?} gpu_cost {} != {} (from {:?})",
                    s.gpu_cost, first.gpu_cost, upgrades[0]
                );
            }
        }

        assert_same_cost("Damage", &damage_upgrades);
        assert_same_cost("Health", &hp_upgrades);
        assert_same_cost("Speed", &speed_upgrades);
    }
}
