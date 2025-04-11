use bevy_math::DVec3;

// Define all bodies in ICRF

pub fn planets_info() -> Vec<(crate::BodyState, crate::BodyInfo)> {
    vec![
        // Sun
        (
            ([DVec3::ZERO, DVec3::ZERO]),
            crate::BodyInfo {
                name: "Sun".to_string(),
                mu: 1.327124400189e20,
                radius: 695700e3,
                j2: 0.0,
                rotational_rate: 0.0,
                tilt: DVec3::new(0.0, -23.5f64.to_radians().sin(), 23.5f64.to_radians().cos()),
                affected: false,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: false,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "sun.png".to_string(),
            },
        ),
        // Mercury
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-5.532624678820648e10, -3.142244505436447e10, -1.105178218162823e10),
                DVec3::new(1.506296861774696e4, -3.451695261553988e4, -2.000019976472707e4),
            ],
            crate::BodyInfo {
                name: "Mercury".to_string(),
                mu: 22031.86855e9,
                radius: 2440.53e3,
                j2: 50.3e-6,
                rotational_rate: 0.00000124001,
                tilt: DVec3::new(0.089,-0.461,0.875),
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "mercury.png".to_string()
            },
        ),
        // Venus
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-1.027356079977646e11, -3.204245144195317e10, -7.918494278719466e9),
                DVec3::new(1.034372730409492e4, -3.035539597798260e4, -1.431342664988898e4),
            ],
            crate::BodyInfo {
                name: "Venus".to_string(),
                mu: 324858.592e9,
                radius: 6051.893e3,
                j2: 4.458e-6,
                rotational_rate: -0.00000029924,
                tilt: DVec3::new(-0.019,-0.388,0.921),
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "venus.png".to_string()
            },
        ),
        // Earth
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-1.464172364494842e11, -2.772532550991856e10, -1.20179117883738e10),
                DVec3::new(5.547160442088672e3, -2.687652548431365e4, -1.165130941973476e4),
            ],
            crate::BodyInfo {
                name: "Earth".to_string(),
                mu: 398600.435436e9,
                radius: 6378.137e3,
                j2: 1082.63e-6,
                rotational_rate: 0.00007292115,
                tilt: DVec3::Z, // By convention ICRF
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "earth.png".to_string()
            },
        ),
        // Mars
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-2.143312584497707e11, 1.130213760581606e11, 5.762147925190119e10),
                DVec3::new(-1.141453632939710e4, -1.719615178206723e4, -7.579590728773417e3),
            ],
            crate::BodyInfo {
                name: "Mars".to_string(),
                mu: 42828.375214e9,
                radius: 3396.19e3,
                j2: 1960.45e-6,
                rotational_rate: 0.0000708822,
                tilt: DVec3::new(0.445,-0.406,0.798),
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "mars.png".to_string()
            },
        ),
        // Jupiter
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(5.562576581984715e10, 7.016788857765751e11, 2.994046193296018e11),
                DVec3::new(-1.319662355577217e4, 1.321933841527028e3, 8.878182563218415e2),
            ],
            crate::BodyInfo {
                name: "Jupiter".to_string(),
                mu: 126686531.900e9,
                radius: 66854e3,
                j2: 14736e-6,
                rotational_rate: 0.00017585,
                tilt: DVec3::new(0.015,-0.434,0.901),
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "jupiter.png".to_string()
            },
        ),
        // Saturn
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(1.423024249523166e12, -1.526699268136860e11, -1.243323925422439e11),
                DVec3::new(7.312819321021123e2, 8.847665914106138e3, 3.622778314873457e3),
            ],
            crate::BodyInfo {
                name: "Saturn".to_string(),
                mu: 37931206.234e9,
                radius: 54364e3,
                j2: 16298e-6,
                rotational_rate: 0.000163785,
                tilt: DVec3::new(0.085,0.073,0.994),
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "saturn.png".to_string()
            },
        ),
        // Uranus
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(1.616488349955040e12, 2.238902602761308e12, 9.576951060641547e11),
                DVec3::new(-5.736655948173632e3, 3.133454447349457e3, 1.453284216289044e3),
            ],
            crate::BodyInfo {
                name: "Uranus".to_string(),
                mu: 5793950.6103e9,
                radius: 25559e3,
                j2: 3343.43e-6,
                rotational_rate: -0.000101237,
                tilt: DVec3::new(-0.214,-0.940,-0.262),
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "uranus.png".to_string()
            },
        ),
        // Neptune
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(4.470300690957564e12, -7.237235901036178e9, -1.142480940335590e11),
                DVec3::new(1.520223426972248e1, 5.064904445004065e3, 2.073412797962369e3),
            ],
            crate::BodyInfo {
                name: "Neptune".to_string(),
                mu: 6835099.97e9,
                radius: 24766e3,
                j2: 3411e-6,
                rotational_rate: 0.000108338,
                tilt: DVec3::new(0.369,-0.622,0.689), // nearly zero obliquity
                affected: true,
                affects: true,
                kepler_parent: 0,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "neptune.png".to_string()
            },
        ),

        // MOONS
        // Luna
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-1.462071124551009e11, -2.746651293891389e10, -1.187506498324082e10),
                DVec3::new(4.690415150659709e3, -2.629349527958113e4, -1.133678648192069e4),
            ],
            crate::BodyInfo {
                name: "Luna".to_string(),
                mu: 4902.800066e9,
                radius: 1738.0e3,
                j2: 202.7e-6,
                rotational_rate: 0.0000026617,
                tilt: DVec3::new(0.0,-0.395,0.918),
                affected: true,
                affects: true,
                kepler_parent: 3,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "luna.png".to_string()
            },
        ),
        // Phobos
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-2.143231508074529e11, 1.130207954358597e11, 5.761660420743605e10),
                DVec3::new(-1.090544043519775e4, -1.525225454690260e4, -6.921594858425093e3),
            ],
            crate::BodyInfo {
                name: "Phobos".to_string(),
                mu: 7.11e5,
                radius: 13.1e3,
                j2: 0.0,
                rotational_rate: 0.0,
                tilt: DVec3::new(0.445,-0.406,0.798),
                affected: true,
                affects: true,
                kepler_parent: 4,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "phobos.png".to_string()
            },
        ),
        // Deimos
        (
            [
                // Position (m) and velocity (m/s)
                DVec3::new(-2.143339367495307e11, 1.130001812861314e11, 5.761177366183721e10),
                DVec3::new(-1.018908957340378e4, -1.709502687832305e4, -8.138962446338176e3),
            ],
            crate::BodyInfo {
                name: "Deimos".to_string(),
                mu: 8.53e4,
                radius: 13.1e3,
                j2: 0.0,
                rotational_rate: 0.0,
                tilt: DVec3::new(0.445,-0.406,0.798),
                affected: true,
                affects: true,
                kepler_parent: 4,
                display_as_keplerian: true,
                orbit_display_id: None,
                body_display_id: None,
                body_display_grid_id: None,
                body_overlay_display_id: None,
                texture: "phobos.png".to_string()
            },
        ),
    ]

    // MOONS

}