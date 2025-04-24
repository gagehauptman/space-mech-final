use bevy::prelude::*;
use bevy_math::{DMat3, DVec3};
use crate::keplerian::{oe_from_rv, pqw_to_inertial_rot, OE};
use crate::StateKeeper;

#[derive(Clone)]
pub struct Interplanetary {
    pub body0: u32,
    pub body1: u32,
    pub body2: u32,
    pub oe0: OE,
    pub oe1: OE,
    pub oe2: OE,
    pub dv1: f64,
    pub dv2: f64,
}

pub fn interplanetary(state_keeper: &ResMut<StateKeeper>, departure_step: u32, arrival_step: u32, body1: u32, body2: u32, short: bool) -> Interplanetary {
    // First, solve for the OE of the transfer orbit
    let delta_step = arrival_step - departure_step;
    let dt = delta_step as f64 * state_keeper.dt;
    let r1 = state_keeper.state.get(&departure_step).unwrap().get(&body1).unwrap()[0];
    let r2 = state_keeper.state.get(&arrival_step).unwrap().get(&body2).unwrap()[0];
    let (v1,v2) = lambert_bate::get_velocities(
        r1.to_array(),
        r2.to_array(),
        dt,
        state_keeper.info.get(&0).unwrap().mu,
        short,
        1e-7,
        100
    ).unwrap();
    let oe0 = oe_from_rv(state_keeper.info.get(&0).unwrap().mu, &[r1, DVec3::from_array(v1)]);
    let v1 = DVec3::from_array(v1);
    let v2 = DVec3::from_array(v2);

    let rp1 = 7000000.0;
    let rp2 = 7000000.0;

    let ([oe1,oe2],[vp1,vp2]) = solve_interplanetary_hyperbolas(state_keeper, departure_step, arrival_step, body1, body2, &v1, &v2, rp1, rp1);

    let v_circ_1 = (state_keeper.info.get(&body1).unwrap().mu / rp1).sqrt();
    let v_circ_2 = (state_keeper.info.get(&body2).unwrap().mu / rp2).sqrt();

    let dv1 = (vp1-v_circ_1).abs();
    let dv2 = (vp2-v_circ_2).abs();

    Interplanetary {body0: 0, body1, body2, oe0, oe1, oe2, dv1, dv2}
}

pub fn solve_interplanetary_hyperbolas(state_keeper: &ResMut<StateKeeper>, departure_step: u32, arrival_step: u32, body1: u32, body2: u32, v1: &DVec3, v2: &DVec3, rp1: f64, rp2: f64) -> ([OE;2],[f64;2]) {
    let r1 = state_keeper.state.get(&departure_step).unwrap().get(&body1).unwrap()[0];
    let r2 = state_keeper.state.get(&arrival_step).unwrap().get(&body2).unwrap()[0];

    // Solve for v_infinity at body1 and body2
    let v_inf1 = v1 - state_keeper.state.get(&departure_step).unwrap().get(&body1).unwrap()[1];
    let v_inf2 = v2 - state_keeper.state.get(&arrival_step).unwrap().get(&body2).unwrap()[1];

    // Solve for energys of 1 and 2
    let E1 = v_inf1.length().powi(2) / 2.0;
    let E2 = v_inf2.length().powi(2) / 2.0;

    // Solve for the hyperbolic periapsis velocities at 1, 2
    let vp_1: f64 = (v_inf1.length().powi(2) + (2.0 * state_keeper.info.get(&body1).unwrap().mu)/rp1).sqrt();
    let vp_2: f64 = (v_inf1.length().powi(2) + (2.0 * state_keeper.info.get(&body1).unwrap().mu)/rp1).sqrt();

    // Calculate the eccentricities of the respective hyperbolas
    let e1 = 1.0 + (rp1 * v_inf1.length().powi(2)) / state_keeper.info.get(&body1).unwrap().mu;
    let e2 = 1.0 + (rp2 * v_inf1.length().powi(2)) / state_keeper.info.get(&body2).unwrap().mu;

    // Calculate the turning angles of the respective hyperbolas
    let turning_angle_1 = 2.0 * (1.0/e1).asin();
    let turning_angle_2 = 2.0 * (1.0/e2).asin();

    // Define the orbit in perifocal frame, to later be rotated into inertial
    let r1_pqw = DVec3::new(rp1, 0.0, 0.0);
    let v1_pqw = DVec3::new(0.0, vp_1, 0.0);
    let r2_pqw = DVec3::new(rp2, 0.0, 0.0);
    let v2_pqw = DVec3::new(0.0, vp_2, 0.0);

    // Solve for direction of v_inf in PQW
    let u_inf1_pqw = DVec3::new(-(turning_angle_1/2.0).cos(), (turning_angle_1/2.0).sin(), 0.0);
    let u_inf2_pqw = DVec3::new(-(turning_angle_2/2.0).cos(), (turning_angle_2/2.0).sin(), 0.0);

    // Define the 'optimal' transfer inclination.
    // TODO: Make this align with CCSFS latitude for earth. doesn't rly matter for mars, not specified in project rubric
    let optimal_inclination_1 = 90.0;
    let optimal_inclination_2 = 90.0;

    // Loop through arg periapsis', find result with inclination closest to optimal_inclination
    let mut optimal_1: [f64;3] = [0.0;3];
    let mut optimal_2: [f64;3] = [0.0;3];
    let mut min1 = 1e5;
    let mut min2 = 1e5;

    for z in 0..360 {
        let ω = (z as f64).to_radians();
        if let Some(f) = solve_euler_angles_from_u_inf(u_inf1_pqw, v_inf1.normalize(), ω) {
            let [Ω1, i1] = f;
            if (optimal_inclination_1 - i1).abs() < min1 {
                optimal_1 = [Ω1, i1, ω];
                min1 = (optimal_inclination_1 - i1).abs()
            }
        }
        if let Some(f) = solve_euler_angles_from_u_inf(u_inf2_pqw, v_inf2.normalize(), ω) {
            let [Ω2, i2] = f;
            if (optimal_inclination_2 - i2).abs() < min2 {
                optimal_2 = [Ω2, i2, ω];
                min2 = (optimal_inclination_2 - i2).abs()
            }
        }
    }

    let R1 = pqw_to_inertial_rot(optimal_1[0], optimal_1[2], optimal_1[1]);
    let R2 = pqw_to_inertial_rot(optimal_2[0], optimal_2[2], optimal_2[1]);

    let rp1 = R1 * r1_pqw;
    let rp2 = R2 * r2_pqw;
    let vp1 = R1 * v1_pqw;
    let vp2 = R2 * v2_pqw;

    ([oe_from_rv(state_keeper.info.get(&body1).unwrap().mu, &[rp1,vp1]), oe_from_rv(state_keeper.info.get(&body1).unwrap().mu, &[rp2,vp2])],[vp1.length(),vp2.length()])
}

pub fn solve_euler_angles_from_u_inf(u_inf_pqw: DVec3, u_inf: DVec3, ω: f64) -> Option<[f64;2]> {
    let A = DMat3::from_axis_angle(DVec3::Z, -ω) * u_inf_pqw;
    if A.y.abs() < 1e-6 {
        return None;
    }
    let i = (u_inf.z / A.y);
    let Ω = (u_inf.x * A.x - u_inf.y * A.x).atan2(u_inf.x * A.x + u_inf.y * A.y * i.cos());
    Some( [Ω, i] )
}

pub fn solve_interplanetaries_for_departure_step(state_keeper: &ResMut<StateKeeper>, departure_step: u32, max_look_ahead: u32) -> Vec<(u32,Interplanetary)> {
    let d: usize = 432;
    let minimum_delta_step = 90*864;
    let maximum_delta_step = max_look_ahead;
    let mut interplanetaries: Vec<(u32,Interplanetary)> = Vec::new();

    for i in (minimum_delta_step..maximum_delta_step).step_by(d) {
        let arrival_step = departure_step + i;
        let ip1 = interplanetary(state_keeper, departure_step, arrival_step, 3, 4, false);
        let ip2 = interplanetary(state_keeper, departure_step, arrival_step, 3, 4, true);
        let dv1 = ip1.dv1 + ip1.dv2;
        let dv2 = ip2.dv1 + ip2.dv2;
        let mut ip = ip2;
        if dv1 < dv2 {
            ip = ip1;
        }

        interplanetaries.push((arrival_step,ip));
    }

    interplanetaries
}