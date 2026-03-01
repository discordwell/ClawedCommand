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
            health: Fixed::from_bits(60 << 16),   // 60
            speed: Fixed::from_bits(7864),         // 0.12
            damage: Fixed::from_bits(4 << 16),     // 4
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 50, gpu_cost: 0, supply_cost: 1, train_time: 50,
        },
        UnitKind::Nuisance => UnitBaseStats {
            health: Fixed::from_bits(80 << 16),    // 80
            speed: Fixed::from_bits(11796),         // 0.18
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 0, supply_cost: 1, train_time: 60,
        },
        UnitKind::Chonk => UnitBaseStats {
            health: Fixed::from_bits(300 << 16),   // 300
            speed: Fixed::from_bits(5242),          // 0.08
            damage: Fixed::from_bits(12 << 16),    // 12
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 20,
            attack_type: AttackType::Melee,
            food_cost: 150, gpu_cost: 25, supply_cost: 3, train_time: 120,
        },
        UnitKind::FlyingFox => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),    // 50
            speed: Fixed::from_bits(14745),         // 0.225
            damage: Fixed::from_bits(6 << 16),     // 6
            range: Fixed::from_bits(2 << 16),      // 2
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 25, supply_cost: 2, train_time: 80,
        },
        UnitKind::Hisser => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),    // 70
            speed: Fixed::from_bits(7864),          // 0.12
            damage: Fixed::from_bits(12 << 16),    // 12 (was 14)
            range: Fixed::from_bits(5 << 16),      // 5
            attack_speed: 13,                      // was 12
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 0, supply_cost: 2, train_time: 80,
        },
        UnitKind::Yowler => UnitBaseStats {
            health: Fixed::from_bits(90 << 16),    // 90
            speed: Fixed::from_bits(9175),          // 0.14
            damage: Fixed::from_bits(5 << 16),     // 5
            range: Fixed::from_bits(4 << 16),      // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 50, supply_cost: 2, train_time: 100,
        },
        UnitKind::Mouser => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),    // 55
            speed: Fixed::from_bits(13107),         // 0.20
            damage: Fixed::from_bits(10 << 16),    // 10
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 25, supply_cost: 1, train_time: 60,
        },
        UnitKind::Catnapper => UnitBaseStats {
            health: Fixed::from_bits(120 << 16),   // 120
            speed: Fixed::from_bits(3932),          // 0.06
            damage: Fixed::from_bits(25 << 16),    // 25
            range: Fixed::from_bits(2 << 16),      // 2
            attack_speed: 30,
            attack_type: AttackType::Ranged,
            food_cost: 200, gpu_cost: 50, supply_cost: 3, train_time: 150,
        },
        UnitKind::FerretSapper => UnitBaseStats {
            health: Fixed::from_bits(65 << 16),    // 65
            speed: Fixed::from_bits(11141),         // 0.17
            damage: Fixed::from_bits(20 << 16),    // 20
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 25,
            attack_type: AttackType::Melee,
            food_cost: 125, gpu_cost: 50, supply_cost: 2, train_time: 100,
        },
        UnitKind::MechCommander => UnitBaseStats {
            health: Fixed::from_bits(500 << 16),   // 500
            speed: Fixed::from_bits(6553),          // 0.10
            damage: Fixed::from_bits(18 << 16),    // 18
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400, gpu_cost: 200, supply_cost: 6, train_time: 250,
        },
        // --- The Murder (Corvids) ---
        UnitKind::MurderScrounger => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),    // 55
            speed: Fixed::from_bits(9175),          // 0.14
            damage: Fixed::from_bits(3 << 16),     // 3
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 50, gpu_cost: 0, supply_cost: 1, train_time: 50,
        },
        UnitKind::Sentinel => UnitBaseStats {
            health: Fixed::from_bits(60 << 16),    // 60
            speed: Fixed::from_bits(10486),         // 0.16
            damage: Fixed::from_bits(12 << 16),    // 12
            range: Fixed::from_bits(6 << 16),      // 6
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 75, gpu_cost: 0, supply_cost: 1, train_time: 60,
        },
        UnitKind::Rookclaw => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),    // 70
            speed: Fixed::from_bits(13107),         // 0.20
            damage: Fixed::from_bits(10 << 16),    // 10
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 0, supply_cost: 1, train_time: 55,
        },
        UnitKind::Magpike => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),    // 55
            speed: Fixed::from_bits(11796),         // 0.18
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(4 << 16),      // 4
            attack_speed: 10,
            attack_type: AttackType::Ranged,
            food_cost: 85, gpu_cost: 25, supply_cost: 2, train_time: 65,
        },
        UnitKind::Magpyre => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),    // 50
            speed: Fixed::from_bits(11141),         // 0.17
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 50, supply_cost: 2, train_time: 90,
        },
        UnitKind::Jaycaller => UnitBaseStats {
            health: Fixed::from_bits(85 << 16),    // 85
            speed: Fixed::from_bits(9175),          // 0.14
            damage: Fixed::from_bits(5 << 16),     // 5
            range: Fixed::from_bits(4 << 16),      // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 100, gpu_cost: 50, supply_cost: 2, train_time: 100,
        },
        UnitKind::Jayflicker => UnitBaseStats {
            health: Fixed::from_bits(60 << 16),    // 60
            speed: Fixed::from_bits(10486),         // 0.16
            damage: Fixed::from_bits(7 << 16),     // 7
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 125, gpu_cost: 50, supply_cost: 2, train_time: 90,
        },
        UnitKind::Dusktalon => UnitBaseStats {
            health: Fixed::from_bits(65 << 16),    // 65
            speed: Fixed::from_bits(13107),         // 0.20
            damage: Fixed::from_bits(15 << 16),    // 15
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 125, gpu_cost: 25, supply_cost: 2, train_time: 80,
        },
        UnitKind::Hootseer => UnitBaseStats {
            health: Fixed::from_bits(100 << 16),   // 100
            speed: Fixed::from_bits(6554),          // 0.10
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(5 << 16),      // 5
            attack_speed: 18,
            attack_type: AttackType::Ranged,
            food_cost: 150, gpu_cost: 50, supply_cost: 3, train_time: 120,
        },
        UnitKind::CorvusRex => UnitBaseStats {
            health: Fixed::from_bits(450 << 16),   // 450
            speed: Fixed::from_bits(6554),          // 0.10
            damage: Fixed::from_bits(16 << 16),    // 16
            range: Fixed::from_bits(4 << 16),      // 4
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400, gpu_cost: 200, supply_cost: 6, train_time: 250,
        },
        // --- Seekers of the Deep (Badgers) ---
        UnitKind::Delver => UnitBaseStats { health: Fixed::from_bits(50 << 16), speed: Fixed::from_bits(6553), damage: Fixed::from_bits(3 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 18, attack_type: AttackType::Melee, food_cost: 50, gpu_cost: 0, supply_cost: 1, train_time: 55 },
        UnitKind::Ironhide => UnitBaseStats { health: Fixed::from_bits(250 << 16), speed: Fixed::from_bits(6553), damage: Fixed::from_bits(16 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 18, attack_type: AttackType::Melee, food_cost: 125, gpu_cost: 0, supply_cost: 2, train_time: 100 }, // speed 0.08→0.10
        UnitKind::Cragback => UnitBaseStats { health: Fixed::from_bits(350 << 16), speed: Fixed::from_bits(3932), damage: Fixed::from_bits(30 << 16), range: Fixed::from_bits(8 << 16), attack_speed: 30, attack_type: AttackType::Ranged, food_cost: 200, gpu_cost: 50, supply_cost: 4, train_time: 150 },
        UnitKind::Warden => UnitBaseStats { health: Fixed::from_bits(150 << 16), speed: Fixed::from_bits(6553), damage: Fixed::from_bits(8 << 16), range: Fixed::from_bits(3 << 16), attack_speed: 15, attack_type: AttackType::Ranged, food_cost: 100, gpu_cost: 0, supply_cost: 2, train_time: 80 }, // gpu 25→0
        UnitKind::Sapjaw => UnitBaseStats { health: Fixed::from_bits(120 << 16), speed: Fixed::from_bits(7864), damage: Fixed::from_bits(18 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 12, attack_type: AttackType::Melee, food_cost: 100, gpu_cost: 0, supply_cost: 2, train_time: 80 },
        UnitKind::Wardenmother => UnitBaseStats { health: Fixed::from_bits(600 << 16), speed: Fixed::from_bits(5242), damage: Fixed::from_bits(22 << 16), range: Fixed::from_bits(3 << 16), attack_speed: 15, attack_type: AttackType::Ranged, food_cost: 450, gpu_cost: 250, supply_cost: 6, train_time: 280 },
        UnitKind::SeekerTunneler => UnitBaseStats { health: Fixed::from_bits(80 << 16), speed: Fixed::from_bits(9175), damage: Fixed::from_bits(6 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 20, attack_type: AttackType::Melee, food_cost: 75, gpu_cost: 25, supply_cost: 1, train_time: 70 },
        UnitKind::Embermaw => UnitBaseStats { health: Fixed::from_bits(90 << 16), speed: Fixed::from_bits(6553), damage: Fixed::from_bits(16 << 16), range: Fixed::from_bits(6 << 16), attack_speed: 15, attack_type: AttackType::Ranged, food_cost: 125, gpu_cost: 25, supply_cost: 2, train_time: 90 },
        UnitKind::Dustclaw => UnitBaseStats { health: Fixed::from_bits(70 << 16), speed: Fixed::from_bits(10486), damage: Fixed::from_bits(12 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 10, attack_type: AttackType::Melee, food_cost: 75, gpu_cost: 0, supply_cost: 1, train_time: 60 },
        UnitKind::Gutripper => UnitBaseStats { health: Fixed::from_bits(160 << 16), speed: Fixed::from_bits(7864), damage: Fixed::from_bits(20 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 8, attack_type: AttackType::Melee, food_cost: 150, gpu_cost: 25, supply_cost: 3, train_time: 120 },
        // --- Croak (Axolotls) ---
        UnitKind::Ponderer => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),    // 55
            speed: Fixed::from_bits(6553),          // 0.10
            damage: Fixed::from_bits(3 << 16),     // 3
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 18,
            attack_type: AttackType::Melee,
            food_cost: 50, gpu_cost: 0, supply_cost: 1, train_time: 50,
        },
        UnitKind::Regeneron => UnitBaseStats {
            health: Fixed::from_bits(80 << 16),    // 80
            speed: Fixed::from_bits(10485),         // 0.16
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 8,
            attack_type: AttackType::Melee,
            food_cost: 70, gpu_cost: 0, supply_cost: 1, train_time: 55,
        },
        UnitKind::Broodmother => UnitBaseStats {
            health: Fixed::from_bits(100 << 16),   // 100
            speed: Fixed::from_bits(6553),          // 0.10
            damage: Fixed::from_bits(4 << 16),     // 4
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 125, gpu_cost: 25, supply_cost: 2, train_time: 100,
        },
        UnitKind::Gulper => UnitBaseStats {
            health: Fixed::from_bits(300 << 16),   // 300
            speed: Fixed::from_bits(4587),          // 0.07
            damage: Fixed::from_bits(14 << 16),    // 14
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 16,
            attack_type: AttackType::Melee,
            food_cost: 160, gpu_cost: 25, supply_cost: 3, train_time: 115,
        },
        UnitKind::Eftsaber => UnitBaseStats {
            health: Fixed::from_bits(60 << 16),    // 60
            speed: Fixed::from_bits(11796),         // 0.18
            damage: Fixed::from_bits(12 << 16),    // 12
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 9,
            attack_type: AttackType::Melee,
            food_cost: 100, gpu_cost: 25, supply_cost: 2, train_time: 80,
        },
        UnitKind::Croaker => UnitBaseStats {
            health: Fixed::from_bits(65 << 16),    // 65
            speed: Fixed::from_bits(6553),          // 0.10
            damage: Fixed::from_bits(16 << 16),    // 16
            range: Fixed::from_bits(6 << 16),      // 6
            attack_speed: 18,
            attack_type: AttackType::Ranged,
            food_cost: 90, gpu_cost: 0, supply_cost: 2, train_time: 70,
        },
        UnitKind::Leapfrog => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),    // 70
            speed: Fixed::from_bits(11141),         // 0.17
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 10,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 0, supply_cost: 1, train_time: 60,
        },
        UnitKind::Shellwarden => UnitBaseStats {
            health: Fixed::from_bits(350 << 16),   // 350
            speed: Fixed::from_bits(3932),          // 0.06
            damage: Fixed::from_bits(6 << 16),     // 6
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 22,
            attack_type: AttackType::Melee,
            food_cost: 175, gpu_cost: 50, supply_cost: 4, train_time: 140,
        },
        UnitKind::Bogwhisper => UnitBaseStats {
            health: Fixed::from_bits(80 << 16),    // 80
            speed: Fixed::from_bits(7209),          // 0.11
            damage: Fixed::from_bits(5 << 16),     // 5
            range: Fixed::from_bits(5 << 16),      // 5
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 125, gpu_cost: 50, supply_cost: 3, train_time: 100,
        },
        // --- The Clawed (Mice) ---
        UnitKind::Nibblet => UnitBaseStats {
            health: Fixed::from_bits(40 << 16),    // 40
            speed: Fixed::from_bits(9830),          // 0.15
            damage: Fixed::from_bits(3 << 16),     // 3
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 30, gpu_cost: 0, supply_cost: 1, train_time: 35,
        },
        UnitKind::Swarmer => UnitBaseStats {
            health: Fixed::from_bits(70 << 16),    // 70 (was 45)
            speed: Fixed::from_bits(13107),         // 0.20 (was 0.15)
            damage: Fixed::from_bits(7 << 16),     // 7 (was 5)
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 6,                       // was 8 (DPS 1.17)
            attack_type: AttackType::Melee,
            food_cost: 40, gpu_cost: 0, supply_cost: 1, train_time: 30,
        },
        UnitKind::Gnawer => UnitBaseStats {
            health: Fixed::from_bits(55 << 16),    // 55
            speed: Fixed::from_bits(6553),          // 0.10
            damage: Fixed::from_bits(6 << 16),     // 6
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 12,
            attack_type: AttackType::Melee,
            food_cost: 50, gpu_cost: 0, supply_cost: 1, train_time: 45,
        },
        UnitKind::Shrieker => UnitBaseStats {
            health: Fixed::from_bits(45 << 16),    // 45
            speed: Fixed::from_bits(9175),          // 0.14
            damage: Fixed::from_bits(8 << 16),     // 8
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 10,
            attack_type: AttackType::Ranged,
            food_cost: 55, gpu_cost: 0, supply_cost: 1, train_time: 40,
        },
        UnitKind::Tunneler => UnitBaseStats {
            health: Fixed::from_bits(60 << 16),    // 60
            speed: Fixed::from_bits(5898),          // 0.09
            damage: Fixed::from_bits(4 << 16),     // 4
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 15,
            attack_type: AttackType::Melee,
            food_cost: 75, gpu_cost: 25, supply_cost: 2, train_time: 70,
        },
        UnitKind::Sparks => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),    // 50 (was 40)
            speed: Fixed::from_bits(11141),         // 0.17
            damage: Fixed::from_bits(10 << 16),    // 10 (was 7)
            range: Fixed::from_bits(4 << 16),      // 4 (was 2)
            attack_speed: 10,                      // was 12 (DPS 1.0)
            attack_type: AttackType::Ranged,
            food_cost: 60, gpu_cost: 0, supply_cost: 1, train_time: 50, // gpu 15→0
        },
        UnitKind::Quillback => UnitBaseStats {
            health: Fixed::from_bits(200 << 16),   // 200
            speed: Fixed::from_bits(3932),          // 0.06
            damage: Fixed::from_bits(10 << 16),    // 10
            range: Fixed::from_bits(1 << 16),      // 1
            attack_speed: 18,
            attack_type: AttackType::Melee,
            food_cost: 100, gpu_cost: 15, supply_cost: 2, train_time: 80,
        },
        UnitKind::Whiskerwitch => UnitBaseStats {
            health: Fixed::from_bits(50 << 16),    // 50
            speed: Fixed::from_bits(7864),          // 0.12
            damage: Fixed::from_bits(4 << 16),     // 4
            range: Fixed::from_bits(4 << 16),      // 4
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 70, gpu_cost: 30, supply_cost: 2, train_time: 65,
        },
        UnitKind::Plaguetail => UnitBaseStats {
            health: Fixed::from_bits(60 << 16),    // 60
            speed: Fixed::from_bits(7209),          // 0.11
            damage: Fixed::from_bits(10 << 16),    // 10 (was 6)
            range: Fixed::from_bits(4 << 16),      // 4 (was 2)
            attack_speed: 12,
            attack_type: AttackType::Ranged,
            food_cost: 45, gpu_cost: 0, supply_cost: 1, train_time: 40,
        },
        UnitKind::WarrenMarshal => UnitBaseStats {
            health: Fixed::from_bits(300 << 16),   // 300
            speed: Fixed::from_bits(5242),          // 0.08
            damage: Fixed::from_bits(12 << 16),    // 12
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 14,
            attack_type: AttackType::Ranged,
            food_cost: 250, gpu_cost: 125, supply_cost: 4, train_time: 200,
        },
        UnitKind::MurkCommander => UnitBaseStats {
            health: Fixed::from_bits(450 << 16),   // 450
            speed: Fixed::from_bits(5898),          // 0.09
            damage: Fixed::from_bits(15 << 16),    // 15
            range: Fixed::from_bits(3 << 16),      // 3
            attack_speed: 15,
            attack_type: AttackType::Ranged,
            food_cost: 400, gpu_cost: 200, supply_cost: 6, train_time: 250,
        },
        // --- LLAMA (Raccoons) ---
        UnitKind::Scrounger => UnitBaseStats {
            health: Fixed::from_bits(55 << 16), speed: Fixed::from_bits(7209), damage: Fixed::from_bits(3 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 15, attack_type: AttackType::Melee, food_cost: 45, gpu_cost: 0, supply_cost: 1, train_time: 45,
        },
        UnitKind::Bandit => UnitBaseStats {
            health: Fixed::from_bits(70 << 16), speed: Fixed::from_bits(12452), damage: Fixed::from_bits(7 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 9, attack_type: AttackType::Melee, food_cost: 65, gpu_cost: 0, supply_cost: 1, train_time: 55,
        },
        UnitKind::HeapTitan => UnitBaseStats {
            health: Fixed::from_bits(280 << 16), speed: Fixed::from_bits(4588), damage: Fixed::from_bits(10 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 22, attack_type: AttackType::Melee, food_cost: 140, gpu_cost: 20, supply_cost: 3, train_time: 110,
        },
        UnitKind::GlitchRat => UnitBaseStats {
            health: Fixed::from_bits(40 << 16), speed: Fixed::from_bits(14418), damage: Fixed::from_bits(5 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 12, attack_type: AttackType::Melee, food_cost: 60, gpu_cost: 15, supply_cost: 1, train_time: 50,
        },
        UnitKind::PatchPossum => UnitBaseStats {
            health: Fixed::from_bits(80 << 16), speed: Fixed::from_bits(8520), damage: Fixed::from_bits(4 << 16), range: Fixed::from_bits(3 << 16), attack_speed: 15, attack_type: AttackType::Ranged, food_cost: 90, gpu_cost: 25, supply_cost: 2, train_time: 80,
        },
        UnitKind::GreaseMonkey => UnitBaseStats {
            health: Fixed::from_bits(65 << 16), speed: Fixed::from_bits(6554), damage: Fixed::from_bits(12 << 16), range: Fixed::from_bits(4 << 16), attack_speed: 14, attack_type: AttackType::Ranged, food_cost: 90, gpu_cost: 0, supply_cost: 2, train_time: 75, // gpu 10→0
        },
        UnitKind::DeadDropUnit => UnitBaseStats {
            health: Fixed::from_bits(50 << 16), speed: Fixed::from_bits(9175), damage: Fixed::from_bits(8 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 12, attack_type: AttackType::Melee, food_cost: 80, gpu_cost: 20, supply_cost: 1, train_time: 65,
        },
        UnitKind::Wrecker => UnitBaseStats {
            health: Fixed::from_bits(100 << 16), speed: Fixed::from_bits(7864), damage: Fixed::from_bits(14 << 16), range: Fixed::from_bits(1 << 16), attack_speed: 10, attack_type: AttackType::Melee, food_cost: 110, gpu_cost: 0, supply_cost: 2, train_time: 85, // gpu 15→0
        },
        UnitKind::DumpsterDiver => UnitBaseStats {
            health: Fixed::from_bits(75 << 16), speed: Fixed::from_bits(7209), damage: Fixed::from_bits(6 << 16), range: Fixed::from_bits(2 << 16), attack_speed: 15, attack_type: AttackType::Ranged, food_cost: 85, gpu_cost: 20, supply_cost: 2, train_time: 70,
        },
        UnitKind::JunkyardKing => UnitBaseStats {
            health: Fixed::from_bits(450 << 16), speed: Fixed::from_bits(5898), damage: Fixed::from_bits(16 << 16), range: Fixed::from_bits(3 << 16), attack_speed: 16, attack_type: AttackType::Ranged, food_cost: 375, gpu_cost: 175, supply_cost: 6, train_time: 230,
        },
        other => unimplemented!("base_stats not yet defined for {other:?}"),
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
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
            assert!(stats.speed > Fixed::ZERO, "{kind:?} should have positive speed");
            assert!(stats.damage > Fixed::ZERO, "{kind:?} should have positive damage");
            assert!(stats.range > Fixed::ZERO, "{kind:?} should have positive range");
            assert!(stats.attack_speed > 0, "{kind:?} should have positive attack_speed");
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
            assert_eq!(stats.attack_type, AttackType::Melee, "{kind:?} should be melee");
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
            assert_eq!(stats.attack_type, AttackType::Ranged, "{kind:?} should be ranged");
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
            UnitKind::MurderScrounger, UnitKind::Sentinel, UnitKind::Rookclaw,
            UnitKind::Magpike, UnitKind::Magpyre, UnitKind::Jaycaller,
            UnitKind::Jayflicker, UnitKind::Dusktalon, UnitKind::Hootseer,
            UnitKind::CorvusRex,
        ];
        for kind in kinds {
            let stats = base_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
            assert!(stats.speed > Fixed::ZERO, "{kind:?} should have positive speed");
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
            assert_eq!(stats.attack_type, AttackType::Melee, "{kind:?} should be melee");
            assert_eq!(stats.range, Fixed::from_bits(1 << 16), "{kind:?} melee should have range 1");
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
            assert_eq!(stats.attack_type, AttackType::Ranged, "{kind:?} should be ranged");
            assert!(stats.range > Fixed::from_bits(1 << 16), "{kind:?} ranged should have range > 1");
        }
    }

    // --- Seekers of the Deep tests ---

    #[test]
    fn all_seekers_have_stats() {
        let kinds = [
            UnitKind::Delver, UnitKind::Ironhide, UnitKind::Cragback,
            UnitKind::Warden, UnitKind::Sapjaw, UnitKind::Wardenmother,
            UnitKind::SeekerTunneler, UnitKind::Embermaw, UnitKind::Dustclaw,
            UnitKind::Gutripper,
        ];
        for kind in kinds {
            let stats = base_stats(kind);
            assert!(stats.health > Fixed::ZERO, "{kind:?} should have positive health");
            assert!(stats.speed > Fixed::ZERO, "{kind:?} should have positive speed");
            assert!(stats.damage > Fixed::ZERO, "{kind:?} should have positive damage");
            assert!(stats.range > Fixed::ZERO, "{kind:?} should have positive range");
            assert!(stats.attack_speed > 0, "{kind:?} should have positive attack_speed");
        }
    }

    #[test]
    fn seekers_melee_units_have_range_one() {
        let melee_kinds = [
            UnitKind::Delver, UnitKind::Ironhide, UnitKind::Sapjaw,
            UnitKind::SeekerTunneler, UnitKind::Dustclaw, UnitKind::Gutripper,
        ];
        for kind in melee_kinds {
            let stats = base_stats(kind);
            assert_eq!(stats.attack_type, AttackType::Melee, "{kind:?} should be melee");
            assert_eq!(stats.range, Fixed::from_bits(1 << 16), "{kind:?} melee should have range 1");
        }
    }

    #[test]
    fn seekers_ranged_units_have_range_gt_one() {
        let ranged_kinds = [
            UnitKind::Cragback, UnitKind::Warden, UnitKind::Wardenmother, UnitKind::Embermaw,
        ];
        for kind in ranged_kinds {
            let stats = base_stats(kind);
            assert_eq!(stats.attack_type, AttackType::Ranged, "{kind:?} should be ranged");
            assert!(stats.range > Fixed::from_bits(1 << 16), "{kind:?} ranged should have range > 1");
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
