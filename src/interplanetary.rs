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

    let rp1 = state_keeper.info.get(&body1).unwrap().radius + 180000.0;
    let rp2 = state_keeper.info.get(&body2).unwrap().radius + 180000.0;

    let ([oe1,oe2],[vp1,vp2]) = solve_interplanetary_hyperbolas(state_keeper, departure_step, arrival_step, body1, body2, &v1, &v2, rp1, rp2);

    let v_circ_1 = (state_keeper.info.get(&body1).unwrap().mu / rp1).sqrt();
    let v_circ_2 = (state_keeper.info.get(&body2).unwrap().mu / rp2).sqrt();

    let dv1 = (vp1-v_circ_1).abs();
    let dv2 = (vp2-v_circ_2).abs();

    Interplanetary {body0: 0, body1, body2, oe0, oe1, oe2, dv1, dv2}
}

pub fn solve_interplanetary_hyperbolas(
    state_keeper: &ResMut<StateKeeper>,
    departure_step: u32,
    arrival_step: u32,
    body1: u32,
    body2: u32,
    v1: &DVec3,
    v2: &DVec3,
    rp1: f64,
    rp2: f64,
) -> ([OE; 2], [f64; 2]) {
    let r1 = state_keeper.state.get(&departure_step).unwrap().get(&body1).unwrap()[0];
    let r2 = state_keeper.state.get(&arrival_step).unwrap().get(&body2).unwrap()[0];
    let v_inf1 = v1 - state_keeper.state.get(&departure_step).unwrap().get(&body1).unwrap()[1];
    let v_inf2 = v2 - state_keeper.state.get(&arrival_step).unwrap().get(&body2).unwrap()[1];
    let mu1 = state_keeper.info.get(&body1).unwrap().mu;
    let mu2 = state_keeper.info.get(&body2).unwrap().mu;
    let vp1 = (v_inf1.length().powi(2) + 2.0 * mu1 / rp1).sqrt();
    let vp2 = (v_inf2.length().powi(2) + 2.0 * mu2 / rp2).sqrt();
    let e1 = 1.0 + rp1 * v_inf1.length().powi(2) / mu1;
    let e2 = 1.0 + rp2 * v_inf2.length().powi(2) / mu2;
    let ta1 = 2.0 * ((1.0 / e1).clamp(-1.0, 1.0)).asin();
    let ta2 = 2.0 * ((1.0 / e2).clamp(-1.0, 1.0)).asin();
    let r1_pqw = DVec3::new(rp1, 0.0, 0.0);
    let v1_pqw = DVec3::new(0.0, vp1, 0.0);
    let r2_pqw = DVec3::new(rp2, 0.0, 0.0);
    let v2_pqw = DVec3::new(0.0, vp2, 0.0);
    let u_inf1_pqw = DVec3::new(-(ta1 / 2.0).cos(), (ta1 / 2.0).sin(), 0.0);
    let u_inf2_pqw = DVec3::new(-(ta2 / 2.0).cos(), (ta2 / 2.0).sin(), 0.0);

    let [Ω1, i1, ω_try1] = (0..360)
        .map(|z| (z as f64).to_radians())
        .find_map(|ωt| solve_euler_angles_from_u_inf(u_inf1_pqw, v_inf1.normalize(), ωt).map(|a| [a[0], a[1], ωt]))
        .unwrap();
    let h1 = r1.cross(v_inf1).normalize();
    let n1 = DVec3::Z.cross(h1).normalize();
    let r1_hat = r1.normalize();
    let cos_ω1 = n1.dot(r1_hat);
    let sin_ω1 = h1.cross(n1).normalize().dot(r1_hat);
    let ω1 = sin_ω1.atan2(cos_ω1);
    let R1 = pqw_to_inertial_rot(Ω1, ω1, i1);

    let [Ω2, i2, ω_try2] = (0..360)
        .map(|z| (z as f64).to_radians())
        .find_map(|ωt| solve_euler_angles_from_u_inf(u_inf2_pqw, v_inf2.normalize(), ωt).map(|a| [a[0], a[1], ωt]))
        .unwrap();
    let h2 = r2.cross(v_inf2).normalize();
    let n2 = DVec3::Z.cross(h2).normalize();
    let r2_hat = r2.normalize();
    let cos_ω2 = n2.dot(r2_hat);
    let sin_ω2 = h2.cross(n2).normalize().dot(r2_hat);
    let ω2 = sin_ω2.atan2(cos_ω2);
    let R2 = pqw_to_inertial_rot(Ω2, ω2, i2);

    let rp1_vec = R1 * r1_pqw;
    let vp1_vec = R1 * v1_pqw;
    let rp2_vec = R2 * r2_pqw;
    let vp2_vec = R2 * v2_pqw;

    (
        [
            oe_from_rv(mu1, &[rp1_vec, vp1_vec]),
            oe_from_rv(mu2, &[rp2_vec, vp2_vec]),
        ],
        [vp1_vec.length(), vp2_vec.length()],
    )
}

pub fn solve_euler_angles_from_u_inf(u_inf_pqw: DVec3, u_inf: DVec3, ω: f64) -> Option<[f64; 2]> {
    let A = DMat3::from_axis_angle(DVec3::Z, -ω) * u_inf_pqw;
    if A.y.abs() < 1e-6 {
        return None;
    }
    let i = u_inf.z / A.y;
    let Ω = (u_inf.x * A.x - u_inf.y * A.x).atan2(u_inf.x * A.x + u_inf.y * A.y * i.cos());
    Some([Ω, i])
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