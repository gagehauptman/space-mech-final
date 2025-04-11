use std::collections::HashMap;
use std::os::linux::raw::stat;
use bevy::prelude::*;
use bevy_math::{DMat3, DVec3};
use rayon::prelude::{IntoParallelIterator, ParallelBridge};
use crate::*;
use rayon::prelude::*;
// Uses an n-body simulation (RK4 implementation, using both point-mass and J2 acceleration) to describe the motion of all bodies in the simulation.
// The transfer orbit is estimated using keplerian orbits, as described in keplerian.rs.

// Calculate gravitational acceleration given r (body1 to planet2),
#[inline(always)]
fn gravitational_acceleration(r: DVec3, mu: f64, body_radius: f64, j2: f64, tilt: DVec3) -> DVec3 {
    let r_norm = r.length();
    let a_central = -mu * r / r_norm.powi(3);
    let r_dot_n = r.dot(tilt);

    let mut a_j2 = DVec3::ZERO;
    // if r_norm < 1e9 {
    //     // J2 acceleration
    //     let factor = 3.0 * j2 * mu * body_radius * body_radius / (2.0 * r_norm.powi(5));
    //     a_j2 = factor * ( (5.0 * (r_dot_n * r_dot_n) / (r_norm * r_norm) - 1.0) * r - 2.0 * r_dot_n * tilt );
    // }
    a_central + a_j2
}

#[inline(always)]
fn compute_derivatives(body_infos: &BodyInfos, body_states: &BodyStates) -> BodyStates {
    let mut derivatives: HashMap<u32, BodyState> = HashMap::new();

    let derivs: Vec<(u32, DVec3, DVec3)> = body_states.iter().map(|x| {
        let id0 = x.0;
        let r0 = x.1[0];
        let v0 = x.1[1];
        let info0 = body_infos.get(&id0).unwrap();
        if info0.affected {
            let acceleration = body_states.iter().map(|y| {
                let id1 = y.0;
                let r1 = y.1[0];
                let v1 = y.1[1];
                let info1 = body_infos.get(&id1).unwrap();
                if id0 != id1 && info1.affects {
                    let r = r0 - r1;
                    return gravitational_acceleration(r, info1.mu, info1.radius, info1.j2, info1.tilt);
                } else {
                    return DVec3::ZERO
                }
            }).fold(DVec3::ZERO, |acc, a| acc + a);
            return (id0.clone(), v0, acceleration)
        } else {
            return (id0.clone(), v0, DVec3::ZERO)
        }
    }).collect();
    for x in derivs.iter() {
        derivatives.insert(x.0, [x.1,x.2]);
    }

    derivatives
}

#[inline(always)]
fn add_scaled_body_states(
    body_states1: &BodyStates,
    body_states2: &BodyStates,
    factor2: f64,
) -> BodyStates {
    let mut ret = body_states1.clone();
    for (id, body_state) in ret.iter_mut() {
        let original = body_states1.get(id).unwrap();
        let delta = body_states2.get(id).unwrap();
        body_state[0] = original[0] + delta[0] * factor2;
        body_state[1] = original[1] + delta[1] * factor2;
    }
    ret
}

#[inline(always)]
pub fn rk4_step(body_infos: &BodyInfos, state: &BodyStates, dt: f64) -> BodyStates {
    // k1 = f(t, y)
    let k1 = compute_derivatives(body_infos, state);
    // y_k2 = y + dt/2 * k1
    let state_k2 = add_scaled_body_states(state, &k1, dt / 2.0);
    let k2 = compute_derivatives(body_infos, &state_k2);
    // y_k3 = y + dt/2 * k2
    let state_k3 = add_scaled_body_states(state, &k2, dt / 2.0);
    let k3 = compute_derivatives(body_infos, &state_k3);
    // y_k4 = y + dt * k3
    let state_k4 = add_scaled_body_states(state, &k3, dt);
    let k4 = compute_derivatives(body_infos, &state_k4);

    let mut new_state: BodyStates = HashMap::new();
    // For each body, update using the weighted sum of the derivatives.
    for (&id, &orig_state) in state.iter() {
        let k1_val = k1.get(&id).unwrap();
        let k2_val = k2.get(&id).unwrap();
        let k3_val = k3.get(&id).unwrap();
        let k4_val = k4.get(&id).unwrap();

        // New position: old position + dt/6 * (k1_vel + 2*k2_vel + 2*k3_vel + k4_vel)
        let new_pos = orig_state[0] + (dt / 6.0) * (k1_val[0] + 2.0 * k2_val[0] + 2.0 * k3_val[0] + k4_val[0]);
        // New velocity: old velocity + dt/6 * (k1_acc + 2*k2_acc + 2*k3_acc + k4_acc)
        let new_vel = orig_state[1] + (dt / 6.0) * (k1_val[1] + 2.0 * k2_val[1] + 2.0 * k3_val[1] + k4_val[1]);
        new_state.insert(id, [new_pos, new_vel]);
    }

    new_state
}
